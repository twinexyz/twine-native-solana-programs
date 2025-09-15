use borsh::{BorshDeserialize, BorshSerialize};
use num_bigint::BigUint;
use serde_json;
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
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{DetailedMessagesBuffer, MessagesReplicator, TransactionType, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_messages_replicator,
            derive_twine_chain_storage, verify_system_program,
        },
        constants::CHAIN_ID,
    },
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ForcedWithdrawalSuccessfulEvent, L1OriginTxPublicValues, NativeTokenVaultData,
            TokenDecimalMappings,
        },
    },
    utils::{
        address_derivation::{
            derive_executed_payouts_pda, derive_native_token_vault_data,
            derive_token_decimal_mappings, verify_derived_address,
        },
        batch_range_provider::batch_range_provider,
        constants::{
            EXECUTED_PAYOUTS_PREFIX, NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX,
        },
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

    let initializer_acc = next_account_info(account_info_iter)?;
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_payouts_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    validate_accounts(
        native_token_vault_data_acc,
        twine_chain_storage_acc,
        token_decimal_mappings_acc,
        system_program,
        detailed_messages_buffer_acc,
        twine_chain_program,
        program_id,
    )?;

    let withdrawal_values =
        decode_withdraw_values(&public_values, receiver_acc.key.to_string().len())?;

    if withdrawal_values.batch_number <= 0 {
        return Err(ProgramCustomError::InvalidBatchNumber.into());
    }

    if withdrawal_values.slot_number <= 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
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

    if withdrawal_values.l1_address != receiver_acc.key.to_string().to_lowercase() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let (expected_executed_payouts_pda, executed_payouts_bump) =
        derive_executed_payouts_pda(program_id, withdrawal_values.nonce);

    if expected_executed_payouts_pda != *executed_payouts_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    
    if executed_payouts_acc.lamports() > 0 {
        msg!("Payout with this nonce has already been executed");
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    }

    let space: usize = 0;
    let rent = Rent::get()?.minimum_balance(space);
    let create_account_ix = system_instruction::create_account(
        initializer_acc.key,
        executed_payouts_acc.key,
        rent,
        space as u64,
        program_id,
    );

    invoke_signed(
        &create_account_ix,
        &[
            initializer_acc.clone(),
            executed_payouts_acc.clone(),
            system_program.clone(),
        ],
        &[&[
            EXECUTED_PAYOUTS_PREFIX.as_bytes(),
            &withdrawal_values.nonce.to_be_bytes(),
            &[executed_payouts_bump],
        ]],
    )?;

    // Deserialize twine_chain_storage_acc
    let twine_chain_storage = {
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    if (withdrawal_values.nonce <= twine_chain_storage.last_copied_message_end_nonce) {
        let (start_nonce, end_nonce) = batch_range_provider(withdrawal_values.nonce).unwrap();
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
        let messages_buffer_data = DetailedMessagesBuffer::deserialize(
            &mut &detailed_messages_buffer_acc.data.borrow()[..],
        )?;
        if !messages_buffer_data
            .messages
            .contains(&Keccak256::digest(&public_values[40..]).into())
        {
            msg!("Error: Provided transaction not present in Detailed Message Buffer.");
            return Err(ProgramCustomError::InvalidTransaction.into());
        };
    }

    if withdrawal_values.batch_number > twine_chain_storage.last_finalized_batch_number {
        return Err(ProgramCustomError::BatchNotFinalized.into());
    };

    if !twine_chain_storage.skip_verification {
        verify_proof(
            &execution_proof,
            &public_values,
            &twine_chain_storage.forced_withdrawal_vkey,
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

    let clock = Clock::get()?;

    let event = ForcedWithdrawalSuccessfulEvent {
        event: "ForcedWithdrawalSuccessful".to_string(),
        nonce: withdrawal_values.nonce,
        l1_receiver: receiver_acc.key.to_string(),
        l1_token: withdrawal_values.l1_token_address,
        chain_id: withdrawal_values.chain_id,
        amount: actual_amount,
        slot_number: clock.slot,
    };

    let serialized_event =
        serde_json::to_string(&event).map_err(|_| ProgramCustomError::FailedToSerializeEvent)?;
    msg!("{}", serialized_event);

    Ok(())
}

fn validate_accounts(
    native_token_vault_data_acc: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    system_program: &AccountInfo,
    detailed_messages_buffer_acc: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_native_token_vault_data, _) = derive_native_token_vault_data(program_id);
    verify_derived_address(
        expected_native_token_vault_data,
        native_token_vault_data_acc,
    )?;

    let (expected_twine_chain_storage, _) = derive_twine_chain_storage(&twine_chain_program_id);
    verify_derived_address(expected_twine_chain_storage, twine_chain_storage_acc)?;

    let (expected_token_decimal_mappings, _) = derive_token_decimal_mappings(program_id);
    verify_derived_address(expected_token_decimal_mappings, token_decimal_mappings_acc)?;

    verify_system_program(system_program);

    let (expected_detailed_message_buffer, _) =
        derive_detailed_messages_buffer(&twine_chain_program_id);
    verify_derived_address(
        expected_detailed_message_buffer,
        detailed_messages_buffer_acc,
    )?;

    if twine_chain_program.key != &twine_chain_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub fn decode_withdraw_values(
    bytes: &[u8],
    l1_address_length: usize,
) -> Result<L1OriginTxPublicValues, ProgramError> {
    const FIXED_PREFIX_LEN: usize = 97;
    const L1_TOKEN_ADDRESS_LEN: usize = 32;
    const L2_ADDRESS_LEN: usize = 42;
    const MIN_AMOUNT_FIELD_SIZE: usize = 1;

    let min_len = FIXED_PREFIX_LEN
        .checked_add(l1_address_length)
        .and_then(|v| v.checked_add(L2_ADDRESS_LEN))
        .and_then(|v| v.checked_add(L1_TOKEN_ADDRESS_LEN))
        .and_then(|v| v.checked_add(L2_ADDRESS_LEN))
        .and_then(|v| v.checked_add(MIN_AMOUNT_FIELD_SIZE))
        .ok_or(ProgramCustomError::PublicValueDecodeFailed)?;

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
    if offset != FIXED_PREFIX_LEN {
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
