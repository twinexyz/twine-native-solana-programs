use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::sysvar::clock::Clock;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{ExecutionMessageBuffer, MessagesReplicator, TransactionType, TwineChainStorage},
    },
    utils::{address_derivation::derive_messages_replicator, constants::CHAIN_ID},
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedRefundsBuffer, ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal,
            NativeTokenVaultData, TokenDecimalMappings, L1OriginTxPublicValues,
        },
    },
    utils::{
        address_derivation::derive_native_token_vault_data,
        batch_range_provider::batch_range_provider,
        constants::{NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn process_native_forced_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let refund_withdrawals_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    let withdraw_values = decode_withdraw_values(&public_values, receiver_acc.key.to_string().len())?;

    if withdraw_values.batch_number <= 0 {
        return Err(ProgramCustomError::InvalidBatchNumber.into());
    }

    if withdraw_values.slot_number <= 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }

    let amount = TokenDecimalMappings::parse_amount_to_u64(&withdraw_values.amount)?;

    if amount <= 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }

    if native_token_vault_acc.lamports() <= amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    }

    if withdraw_values.l1_token_address != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&withdraw_values.l2_token_address)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if withdraw_values.l1_address != receiver_acc.key.to_string() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let (start_nonce, end_nonce) = batch_range_provider(withdraw_values.nonce).unwrap();
    let (expected_message_replicator, _bump_seed) =
        derive_messages_replicator(&twine_chain_program_id, start_nonce, end_nonce);

    if *messages_replicator_acc.key != expected_message_replicator {
        msg!("Error: Incorrect MessagesReplicator PDA provided.");
        return Err(ProgramError::InvalidArgument);
    }

    let messages_replicator =
        MessagesReplicator::deserialize(&mut &messages_replicator_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !messages_replicator
        .messages
        .contains(&withdraw_values.calculate_deposit_hash())
    {
        msg!("Error: Provided transaction not present in PDA.");
        return Err(ProgramCustomError::InvalidTransaction.into());
    };

    // Deserialize twine_chain_storage_acc
    let twine_chain_storage = {
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    if withdraw_values.batch_number > twine_chain_storage.last_finalized_batch_number {
        return Err(ProgramCustomError::BatchNotFinalized.into());
    };

    // encoding public input structure to get public input
    if !twine_chain_storage.skip_verification {
        verify_proof(
            &execution_proof,
            &public_values,
            &twine_chain_storage.execution_vkey,
            GROTH16_VK_4_0_0_RC3_BYTES,
        )
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    }

    let token_decimal_mappings =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let decimal_mapping = token_decimal_mappings
        .get_mapping(&withdraw_values.l1_token_address)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let converted_amount = TokenDecimalMappings::convert_l2_to_l1(
        &withdraw_values.amount,
        decimal_mapping.l2_decimals,
        decimal_mapping.l1_decimals,
    )?;

    let actual_amount = TokenDecimalMappings::parse_amount_to_u64(&converted_amount)?;
    let mut flag = false;

    let mut executed_refunds_buffer =
        ExecutedRefundsBuffer::deserialize(&mut &refund_withdrawals_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    // For refunds
    if withdraw_values.nonce < executed_refunds_buffer.refund_nonce_lower_bound {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    if executed_refunds_buffer
        .executed_refund_nonces
        .contains(&withdraw_values.nonce)
    {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    // Native token (SOL) withdrawal
    process_native_token_withdrawal(
        &program_id,
        &native_token_vault_acc,
        &native_token_vault_data_acc,
        &system_program,
        &receiver_acc,
        actual_amount,
    )?;
    executed_refunds_buffer
        .executed_refund_nonces
        .push(withdraw_values.nonce);

    executed_refunds_buffer.post_withdrawal_processing();
    let clock = Clock::get()?;

    msg!(
    "EVENT:REFUND_SUCCESSFUL: nonce={}, l1_receiver_address={}, l1_token_address={}, chain_id={}, amount={}, slot={}",
    withdraw_values.nonce,
    withdraw_values.l1_address,
    withdraw_values.l1_token_address,
    withdraw_values.chain_id,
    actual_amount,
    clock.slot
);
    Ok(())
}

pub fn decode_withdraw_values(
    bytes: &[u8],
    l1_address_length: usize,
) -> Result<L1OriginTxPublicValues, ProgramError> {
    const MIN_LEN: usize = 168;
    if bytes.len() < MIN_LEN {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }
    let batch_hash: [u8; 32] = bytes[0..32].try_into().unwrap();
    let batch_number = u64::from_be_bytes(bytes[32..40].try_into().unwrap());
    let txn_type = TransactionType::try_from(bytes[40])?;
    let nonce = u64::from_be_bytes(bytes[40..48].try_into().unwrap());
    let chain_id = u64::from_be_bytes(bytes[48..56].try_into().unwrap());
    let slot_number = u64::from_be_bytes(bytes[56..64].try_into().unwrap());

    let offset = |start: usize| start + l1_address_length;
    let l1_address = decode_string_field(&bytes[48..offset(48)])?;
    let l2_address = decode_string_field(&bytes[offset(48)..offset(80)])?;
    let l1_token_address = decode_string_field(&bytes[offset(80)..offset(112)])?;
    let l2_token_address = decode_string_field(&bytes[offset(112)..offset(144)])?;
    let amount = decode_string_field(&bytes[offset(144)..152])?;
    let message: Vec<u8> = bytes[offset(152)..].try_into().unwrap();

    Ok(L1OriginTxPublicValues {
        batch_hash,
        batch_number,
        txn_type,
        nonce,
        chain_id,
        slot_number,
        l1_address,
        l2_address,
        l1_token_address,
        l2_token_address,
        amount,
        message,
    })
}

// Helper function to decode string fields
fn decode_string_field(bytes: &[u8]) -> Result<String, ProgramError> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok(s.trim_end_matches('\0').to_string()),
        Err(_) => Err(ProgramCustomError::PublicValueDecodeFailed.into()),
    }
}

fn process_native_token_withdrawal<'info>(
    program_id: &Pubkey,
    native_token_vault: &AccountInfo<'info>,
    native_token_vault_data: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    receiver: &AccountInfo<'info>,
    amount: u64,
) -> ProgramResult {
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }

    if native_token_vault.lamports() <= amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    };

    let native_vault_seeds = &[NATIVE_TOKEN_VAULT_PREFIX.as_bytes()];
    let (native_vault_key, native_vault_bump) =
        Pubkey::find_program_address(native_vault_seeds, program_id);

    if native_vault_key != *native_token_vault.key {
        return Err(ProgramCustomError::InvalidAccount.into());
    }

    let seeds = &[NATIVE_TOKEN_VAULT_PREFIX.as_bytes(), &[native_vault_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_instruction =
        solana_program::system_instruction::transfer(native_token_vault.key, &receiver.key, amount);

    invoke_signed(
        &transfer_instruction,
        &[
            native_token_vault.clone(),
            receiver.clone(),
            system_program.clone(),
        ],
        signer_seeds,
    )?;

    let mut vault_data =
        NativeTokenVaultData::deserialize(&mut &native_token_vault_data.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;
    vault_data
        .serialize(&mut &mut native_token_vault_data.data.borrow_mut()[..])
        .map_err(|_| ProgramError::AccountDataTooSmall)?;

    Ok(())
}
