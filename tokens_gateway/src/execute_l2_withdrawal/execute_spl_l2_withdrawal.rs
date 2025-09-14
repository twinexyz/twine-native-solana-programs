use borsh::{BorshDeserialize, BorshSerialize};
use num_bigint::BigUint;
use serde_json;
use solana_program::sysvar::clock::Clock;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use spl_token::instruction as token_instruction;
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{RoleType, TwineChainRoleManager, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_twine_chain_role_manager, derive_twine_chain_storage, verify_derived_address,
        },
        constants::CHAIN_ID,
    },
    ID as twine_chain_program_id,
};

use crate::utils::address_derivation::{
    derive_executed_withdrawals_buffer, derive_spl_tokens_vault_data, derive_spl_vault_authority,
    derive_token_decimal_mappings,
};
use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedWithdrawalsBuffer, L2WithdrawExecutedEvent, L2WithdrawValues,
            SplTokensVaultData, TokenDecimalMappings,
        },
    },
    utils::{
        constants::{SPL_AUTH_PREFIX, SPL_TOKENS_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn execute_spl_l2_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let initializer_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let vault_authority_acc = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    validate_accounts(
        initializer_acc,
        spl_tokens_vault_data_acc,
        vault_authority_acc,
        twine_chain_storage_acc,
        executed_withdrawals_buffer_acc,
        token_decimal_mappings_acc,
        role_manager_acc,
        twine_chain_program,
        program_id,
    )?;

    let withdrawal_values = decode_l2_withdraw_values(
        &public_values,
        receiver_acc.key.to_string().len(),
        mint.key.to_string().len(),
    )?;
    if withdrawal_values.batch_number <= 0 {
        return Err(ProgramCustomError::InvalidBatchNumber.into());
    }
    let amount = TokenDecimalMappings::parse_amount_to_biguint(&withdrawal_values.amount)?;
    if amount <= BigUint::ZERO {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if withdrawal_values.l1_token_address == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }
    if withdrawal_values.l1_token_address != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidArgument.into());
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
    msg!("the actual amount output {}", actual_amount);
    let mut executed_withdrawal_buffer = ExecutedWithdrawalsBuffer::deserialize(
        &mut &executed_withdrawals_buffer_acc.data.borrow()[..],
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    if withdrawal_values.nonce < executed_withdrawal_buffer.withdrawal_nonce_lower_bound {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    if executed_withdrawal_buffer
        .executed_withdrawal_nonces
        .binary_search(&withdrawal_values.nonce)
        .is_ok()
    {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    }

    // Spl Token withdrawal
    process_spl_token_withdrawal(
        &program_id,
        &spl_tokens_vault_acc,
        &spl_tokens_vault_data_acc,
        &vault_authority_acc,
        &mint,
        &token_program,
        &receiver_acc,
        actual_amount,
    )?;
    executed_withdrawal_buffer
        .executed_withdrawal_nonces
        .push(withdrawal_values.nonce);
    executed_withdrawal_buffer.post_withdrawal_processing();

    executed_withdrawal_buffer
        .serialize(&mut &mut executed_withdrawals_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

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
    spl_tokens_vault_data_acc: &AccountInfo,
    vault_authority_acc: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    executed_withdrawals_buffer_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let (expected_spl_tokens_vault_data_acc, _) = derive_spl_tokens_vault_data(program_id);
    verify_derived_address(
        expected_spl_tokens_vault_data_acc,
        spl_tokens_vault_data_acc,
    )?;

    let (expected_vault_authority_acc, _) = derive_spl_vault_authority(program_id);
    verify_derived_address(expected_vault_authority_acc, vault_authority_acc)?;

    let (expected_twine_chain_Storage, _) = derive_twine_chain_storage(&twine_chain_program_id);
    verify_derived_address(expected_twine_chain_Storage, twine_chain_storage_acc)?;

    let (expected_executed_withdrawals_buffer, _) = derive_executed_withdrawals_buffer(program_id);
    verify_derived_address(
        expected_executed_withdrawals_buffer,
        executed_withdrawals_buffer_acc,
    )?;

    let (expected_role_manager, _) = derive_twine_chain_role_manager(&twine_chain_program_id);

    verify_derived_address(expected_role_manager, role_manager_acc)?;

    let (expected_token_decimal_mappings, _) = derive_token_decimal_mappings(program_id);
    verify_derived_address(expected_token_decimal_mappings, token_decimal_mappings_acc);

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
    l1_token_address_length: usize,
) -> Result<L2WithdrawValues, ProgramError> {
    const MIN_LEN: usize = 165;
    const PREFIX_LEN: usize = 48;
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
    let l1_token_end = offset + l1_token_address_length;
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

fn process_spl_token_withdrawal<'info>(
    program_id: &Pubkey,
    spl_tokens_vault: &AccountInfo<'info>,
    spl_tokens_vault_data_acc: &AccountInfo<'info>,
    vault_authority: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    receiver: &AccountInfo<'info>,
    amount: u64,
) -> ProgramResult {
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }

    let vault_token_account =
        spl_token::state::Account::unpack(&mut &spl_tokens_vault.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if vault_token_account.amount < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    let spl_data_seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()];
    let (spl_data_key, _) = Pubkey::find_program_address(spl_data_seeds, program_id);

    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let spl_vault_seeds = &[SPL_AUTH_PREFIX.as_bytes()];
    let (_, spl_vault_bump) = Pubkey::find_program_address(spl_vault_seeds, program_id);
    let seeds = &[SPL_AUTH_PREFIX.as_bytes(), &[spl_vault_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_instruction = token_instruction::transfer(
        &spl_token::id(),
        &spl_tokens_vault.key,
        &receiver.key,
        &vault_authority.key,
        &[],
        amount,
    )?;

    invoke_signed(
        &transfer_instruction,
        &[
            spl_tokens_vault.clone(),
            receiver.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        signer_seeds,
    )?;
    let receiver_token_account = spl_token::state::Account::unpack(&receiver.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut spl_tokens_vault_data =
        SplTokensVaultData::deserialize(&mut &spl_tokens_vault_data_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    spl_tokens_vault_data.update_withdraw(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut &mut spl_tokens_vault_data_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}
