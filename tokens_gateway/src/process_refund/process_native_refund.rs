use borsh::{BorshDeserialize, BorshSerialize};
use num_bigint::BigUint;
use sha3::{Digest, Keccak256};
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
        state::{
            ExecutionMessageBuffer, MessagesBuffer, MessagesReplicator, TransactionType,
            TwineChainStorage,
        },
    },
    utils::{address_derivation::derive_messages_replicator, constants::CHAIN_ID},
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedPayoutsBuffer, ExecutedWithdrawalsBuffer,
            L1OriginTxPublicValues, NativeTokenVaultData, TokenDecimalMappings,
        },
    },
    utils::{
        address_derivation::derive_native_token_vault_data,
        batch_range_provider::batch_range_provider,
        constants::{NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn process_native_refund(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_payouts_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    let refund_values = decode_refund_values(&public_values, receiver_acc.key.to_string().len())?;

    if refund_values.batch_number <= 0 {
        return Err(ProgramCustomError::InvalidBatchNumber.into());
    }

    if refund_values.slot_number <= 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }

    let amount = TokenDecimalMappings::parse_amount_to_biguint(&refund_values.amount)?;
    if amount <= BigUint::ZERO {
        return Err(ProgramCustomError::InvalidAmount.into());
    }

    if refund_values.l1_token_address != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&refund_values.l2_token_address)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if refund_values.l1_address != receiver_acc.key.to_string() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }
    let mut executed_payouts_buffer =
        ExecutedPayoutsBuffer::deserialize(&mut &executed_payouts_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // For refunds
    if refund_values.nonce < executed_payouts_buffer.payout_nonce_lower_bound {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    if executed_payouts_buffer
        .executed_payout_nonces
        .contains(&refund_values.nonce)
    {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    // Deserialize twine_chain_storage_acc
    let twine_chain_storage = {
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    if (refund_values.nonce <= twine_chain_storage.last_copied_message_end_nonce) {
        let (start_nonce, end_nonce) = batch_range_provider(refund_values.nonce).unwrap();
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
            .contains(&Keccak256::digest(&public_values[40..]).into())
        {
            msg!("Error: Provided transaction not present in PDA.");
            return Err(ProgramCustomError::InvalidTransaction.into());
        };
    } else {
        let messages_buffer_data =
            MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])?;
        if !messages_buffer_data
            .messages
            .contains(&Keccak256::digest(&public_values[40..]).into())
        {
            msg!("Error: Provided transaction not present in MessageBuffer.");
            return Err(ProgramCustomError::InvalidTransaction.into());
        };
    }

    if refund_values.batch_number > twine_chain_storage.last_finalized_batch_number {
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
        .get_mapping(&refund_values.l1_token_address)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let converted_amount = TokenDecimalMappings::convert_l2_to_l1(
        &refund_values.amount,
        decimal_mapping.l2_decimals,
        decimal_mapping.l1_decimals,
    )?;

    let actual_amount = TokenDecimalMappings::parse_amount_to_u64(&converted_amount)?;

    if native_token_vault_acc.lamports() <= actual_amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    }

    // Native token (SOL) withdrawal
    process_native_token_withdrawal(
        &program_id,
        &native_token_vault_acc,
        &native_token_vault_data_acc,
        &system_program,
        &receiver_acc,
        actual_amount,
    )?;

    executed_payouts_buffer
        .executed_payout_nonces
        .push(refund_values.nonce);

    executed_payouts_buffer.post_withdrawal_processing();
    let clock = Clock::get()?;

    msg!(
    "EVENT:REFUND_SUCCESSFUL: nonce={}, l1_receiver_address={}, l1_token_address={}, chain_id={}, amount={}, slot={}",
    refund_values.nonce,
    refund_values.l1_address,
    refund_values.l1_token_address,
    refund_values.chain_id,
    actual_amount,
    clock.slot
);

    Ok(())
}
pub fn decode_refund_values(
    bytes: &[u8],
    l1_address_length: usize,
) -> Result<L1OriginTxPublicValues, ProgramError> {
    const PREFIX_LEN: usize = 97;
    const L1_TOKEN_ADDRESS_LEN: usize = 32;
    const L2_ADDRESS_LEN: usize = 42;

    let min_len = 233;

    if bytes.len() < min_len {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }

    let mut offset = 0;
    let take = |len: usize, offset: &mut usize| -> Result<&[u8], ProgramError> {
        let start = *offset;
        let end = start
            .checked_add(len)
            .ok_or(ProgramCustomError::PublicValueDecodeFailed)?;
        if end > bytes.len() {
            return Err(ProgramCustomError::PublicValueDecodeFailed.into());
        }
        *offset = end;
        Ok(&bytes[start..end])
    };
    let batch_hash = take(32, &mut offset)?
        .try_into()
        .map_err(|_| ProgramCustomError::PublicValueDecodeFailed)?;
    let batch_number = u64::from_be_bytes(
        take(8, &mut offset)?
            .try_into()
            .map_err(|_| ProgramCustomError::PublicValueDecodeFailed)?,
    );
    let txn_type = TransactionType::try_from(take(1, &mut offset)?[0])?;
    let nonce = u64::from_be_bytes(
        take(8, &mut offset)?
            .try_into()
            .map_err(|_| ProgramCustomError::PublicValueDecodeFailed)?,
    );
    let chain_id = u64::from_be_bytes(
        take(8, &mut offset)?
            .try_into()
            .map_err(|_| ProgramCustomError::PublicValueDecodeFailed)?,
    );
    let slot_number = u64::from_be_bytes(
        take(8, &mut offset)?
            .try_into()
            .map_err(|_| ProgramCustomError::PublicValueDecodeFailed)?,
    );
    let message = take(32, &mut offset)?
        .try_into()
        .map_err(|_| ProgramCustomError::PublicValueDecodeFailed)?;
    if offset != PREFIX_LEN {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }

    let l1_address = decode_string_field(take(l1_address_length, &mut offset)?)?;
    let l2_address = decode_string_field(take(L2_ADDRESS_LEN, &mut offset)?)?;
    let l1_token_address = decode_string_field(take(L1_TOKEN_ADDRESS_LEN, &mut offset)?)?;
    let l2_token_address = decode_string_field(take(L2_ADDRESS_LEN, &mut offset)?)?;

    if offset >= bytes.len() {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }
    let remaining = &bytes[offset..];
    let amount = decode_string_field(remaining)?;
    Ok(L1OriginTxPublicValues {
        batch_hash,
        batch_number,
        txn_type,
        nonce,
        chain_id,
        slot_number,
        message,
        l1_address,
        l2_address,
        l1_token_address,
        l2_token_address,
        amount,
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
