use borsh::{BorshDeserialize, BorshSerialize};
use num_bigint::BigUint;
use serde_json;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use twine_chain::core::{
    instruction::TwineChainInstruction,
    state::{ExecutionMessageBuffer, TwineChainStorage},
};
use twine_chain::utils::constants::CHAIN_ID;

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedWithdrawalsBuffer, L2WithdrawExecutedEvent, L2WithdrawValues,
            NativeTokenVaultData, TokenDecimalMappings,
        },
    },
    utils::{
        address_derivation::derive_native_token_vault_data,
        constants::{NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn execute_native_l2_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    let withdrawal_values =
        decode_l2_withdraw_values(&public_values, receiver_acc.key.to_string().len())?;
    if withdrawal_values.batch_number <= 0 {
        return Err(ProgramCustomError::InvalidBatchNumber.into());
    }
    let amount = TokenDecimalMappings::parse_amount_to_biguint(&withdrawal_values.amount)?;
    if amount <= BigUint::ZERO {
        return Err(ProgramCustomError::InvalidAmount.into());
    }

    if withdrawal_values.l1_token_address != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&withdrawal_values.l2_token_address)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if withdrawal_values.l1_receiver_address != receiver_acc.key.to_string() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }
    // Deserialize twine_chain_storage_acc
    let twine_chain_storage = {
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    if withdrawal_values.batch_number > twine_chain_storage.last_finalized_batch_number {
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
        .get_mapping(&withdrawal_values.l1_token_address)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let converted_amount = TokenDecimalMappings::convert_l2_to_l1(
        &withdrawal_values.amount,
        decimal_mapping.l2_decimals,
        decimal_mapping.l1_decimals,
    )?;

    let actual_amount = TokenDecimalMappings::parse_amount_to_u64(&converted_amount)?;

    if native_token_vault_acc.lamports() <= actual_amount {
        msg!("Insufficient fund in native token vault");
        return Err(ProgramCustomError::InsufficientFunds.into());
    }

    let mut executed_withdrawal_buffer = ExecutedWithdrawalsBuffer::deserialize(
        &mut &executed_withdrawals_buffer_acc.data.borrow()[..],
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    // For L2 initiated withdrawals
    if withdrawal_values.nonce < executed_withdrawal_buffer.withdrawal_nonce_lower_bound {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    if executed_withdrawal_buffer
        .executed_withdrawal_nonces
        .contains(&withdrawal_values.nonce)
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
    executed_withdrawal_buffer
        .executed_withdrawal_nonces
        .push(withdrawal_values.nonce);
    executed_withdrawal_buffer.post_withdrawal_processing();

    let clock = Clock::get()?;

    let event = L2WithdrawExecutedEvent {
        event: "L2WithdrawExecuted".to_string(),
        nonce: withdrawal_values.nonce,
        l1_token: withdrawal_values.l1_token_address,
        l2_token: withdrawal_values.l2_token_address,
        l1_receiver: withdrawal_values.l1_receiver_address,
        chain_id: CHAIN_ID,
        amount: actual_amount,
        slot_number: clock.slot,
    };

    let serialized_event =
        serde_json::to_string(&event).map_err(|_| ProgramError::InvalidInstructionData)?;
    msg!("{}", serialized_event);

    Ok(())
}

pub fn decode_l2_withdraw_values(
    bytes: &[u8],
    l1_receiver_address_length: usize,
) -> Result<L2WithdrawValues, ProgramError> {
    const MIN_LEN: usize = 168;
    const PREFIX_LEN: usize = 48;
    const L1_TOKEN_ADDRESS_LEN: usize = 32;
    const L2_TOKEN_ADDRESS_LEN: usize = 42;

    if bytes.len() < MIN_LEN {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }

    // Extract batchNumber (uint64) from bytes[0:8]
    let batch_number = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
    // Extract nonce (uint64) from bytes[8:16]
    let nonce = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    // Extract batchHash (bytes32) from bytes[16:48]
    let mut batch_hash = [0u8; 32];
    batch_hash.copy_from_slice(&bytes[16..48]);

    let mut offset = PREFIX_LEN;

    let l1_receiver_end = offset + l1_receiver_address_length;
    let l1_receiver_address = decode_string_field(&bytes[offset..l1_receiver_end])?;

    offset = l1_receiver_end;
    let l1_token_end = offset + L1_TOKEN_ADDRESS_LEN;
    let l1_token_address = decode_string_field(&bytes[offset..l1_token_end])?;

    offset = l1_token_end;
    let l2_token_end = offset + L2_TOKEN_ADDRESS_LEN;
    let l2_token_address = decode_string_field(&bytes[offset..l2_token_end])?;

    offset = l2_token_end;
    let amount = decode_string_field(&bytes[offset..])?;

    Ok(L2WithdrawValues {
        batch_number,
        nonce,
        batch_hash,
        l1_receiver_address,
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
