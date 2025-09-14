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
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};

use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{RoleType, TwineChainRoleManager, TwineChainStorage},
    },
    utils::{
        address_derivation::{derive_twine_chain_role_manager, derive_twine_chain_storage},
        constants::CHAIN_ID,
    },
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            L2WithdrawExecutedEvent, L2WithdrawValues, NativeTokenVaultData, TokenDecimalMappings,
        },
    },
    utils::{
        address_derivation::{
            derive_executed_withdrawals_pda, derive_native_token_vault,
            derive_native_token_vault_data, derive_token_decimal_mappings, verify_derived_address,
            verify_system_program,
        },
        constants::{
            EXECUTED_WITHDRAWALS_PREFIX, NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX,
        },
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

    let initializer_acc = next_account_info(account_info_iter)?;
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_withdrawals_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    validate_accounts(
        initializer_acc,
        native_token_vault_acc,
        native_token_vault_data_acc,
        twine_chain_storage_acc,
        token_decimal_mappings_acc,
        role_manager_acc,
        system_program,
        twine_chain_program,
        program_id,
    )?;

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

    let (expected_executed_withdrawals_pda, executed_withdrawals_bump) =
        derive_executed_withdrawals_pda(program_id, withdrawal_values.nonce);

    if expected_executed_withdrawals_pda != *executed_withdrawals_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let space: usize = 0;
    let rent = Rent::get()?.minimum_balance(space);
    let create_account_ix = system_instruction::create_account(
        initializer_acc.key,
        executed_withdrawals_acc.key,
        rent,
        space as u64,
        program_id, 
    );

    invoke_signed(
        &create_account_ix,
        &[
            initializer_acc.clone(),
            executed_withdrawals_acc.clone(),
            system_program.clone(),
        ],
        &[&[
            EXECUTED_WITHDRAWALS_PREFIX.as_bytes(),
            &withdrawal_values.nonce.to_le_bytes(),
            &[executed_withdrawals_bump],
        ]],
    )?;

    // Deserialize twine_chain_storage_acc
    let twine_chain_storage = {
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    if withdrawal_values.batch_number > twine_chain_storage.last_finalized_batch_number {
        return Err(ProgramCustomError::BatchNotFinalized.into());
    };

    if !twine_chain_storage.skip_verification {
        verify_proof(
            &execution_proof,
            &public_values,
            &twine_chain_storage.l2_withdrawal_vkey,
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

    // Native token (SOL) withdrawal
    process_native_token_withdrawal(
        program_id,
        native_token_vault_acc,
        native_token_vault_data_acc,
        system_program,
        receiver_acc,
        actual_amount,
    )?;

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
        serde_json::to_string(&event).map_err(|_| ProgramCustomError::FailedToSerializeEvent)?;
    msg!("{}", serialized_event);

    Ok(())
}

fn validate_accounts(
    initializer_acc: &AccountInfo,
    native_token_vault_acc: &AccountInfo,
    native_token_vault_data_acc: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    system_program: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_native_token_vault, _) = derive_native_token_vault(program_id);
    verify_derived_address(expected_native_token_vault, native_token_vault_acc)?;

    let (expected_native_token_vault_data, _) = derive_native_token_vault_data(program_id);
    verify_derived_address(
        expected_native_token_vault_data,
        native_token_vault_data_acc,
    )?;

    let (expected_twine_chain_storage, _) = derive_twine_chain_storage(&twine_chain_program_id);
    verify_derived_address(expected_twine_chain_storage, twine_chain_storage_acc)?;

    let (expected_role_manager, _) = derive_twine_chain_role_manager(&twine_chain_program_id);
    verify_derived_address(expected_role_manager, role_manager_acc)?;

    let (expected_token_decimal_mapping, _) = derive_token_decimal_mappings(program_id);
    verify_derived_address(expected_token_decimal_mapping, token_decimal_mappings_acc)?;

    verify_system_program(system_program);

    if twine_chain_program.key != &twine_chain_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(initializer_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(())
}

pub fn decode_l2_withdraw_values(
    bytes: &[u8],
    l1_receiver_address_length: usize,
) -> Result<L2WithdrawValues, ProgramError> {
    const MIN_LEN: usize = 165;
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
        solana_program::system_instruction::transfer(native_token_vault.key, receiver.key, amount);

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
