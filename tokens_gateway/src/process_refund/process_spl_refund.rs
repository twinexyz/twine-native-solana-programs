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
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use spl_token::instruction as token_instruction;
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
            ExecutedPayoutsBuffer, FinalizeInputWithdrawal, L1OriginTxPublicValues,
            L2WithdrawValues, SplTokensVaultData, TokenDecimalMappings,
        },
    },
    utils::{
        batch_range_provider::batch_range_provider,
        constants::{SPL_AUTH_PREFIX, SPL_TOKENS_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn process_spl_refund(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let vault_authority_acc = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_payouts_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    let refund_values = decode_refund_values(
        &public_values,
        receiver_acc.key.to_string().len(),
        mint.key.to_string().len(),
    )?;
    if refund_values.batch_number <= 0 {
        return Err(ProgramCustomError::InvalidBatchNumber.into());
    }
    let amount = TokenDecimalMappings::parse_amount_to_biguint(&refund_values.amount)?;
    if amount <= BigUint::ZERO {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if refund_values.l1_token_address == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }
    if refund_values.l1_token_address != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidArgument.into());
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

    let mut executed_refunds_buffer =
        ExecutedPayoutsBuffer::deserialize(&mut &executed_payouts_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if refund_values.nonce < executed_refunds_buffer.payout_nonce_lower_bound {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

    if executed_refunds_buffer
        .executed_payout_nonces
        .contains(&refund_values.nonce)
    {
        return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
    };

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
    executed_refunds_buffer
        .executed_payout_nonces
        .push(refund_values.nonce);
    executed_refunds_buffer.post_withdrawal_processing();

    let clock = Clock::get()?;

    msg!(
    "EVENT:SPL_REFUND_SUCCESSFUL: nonce={}, l1_receiver_address={}, l1_token_address={}, chain_id={}, amount={}, slot={}",
    refund_values.nonce,
    refund_values.l1_address,
    refund_values.l1_token_address,
    CHAIN_ID,
    actual_amount,
    clock.slot
);
    Ok(())
}

pub fn decode_refund_values(
    bytes: &[u8],
    l1_address_length: usize,
    l1_token_address_length: usize,
) -> Result<L1OriginTxPublicValues, ProgramError> {
    const FIXED_PREFIX_LEN: usize = 97;
    const L2_ADDRESS_LEN: usize = 42;
    const L2_TOKEN_ADDRESS_LEN: usize = 42;

    let min_len = FIXED_PREFIX_LEN
        .checked_add(l1_address_length)
        .and_then(|v| v.checked_add(L2_ADDRESS_LEN))
        .and_then(|v| v.checked_add(l1_token_address_length))
        .and_then(|v| v.checked_add(L2_TOKEN_ADDRESS_LEN))
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

    // --- Fixed fields ---
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
    let l1_token_address = decode_string_field(take(l1_token_address_length, &mut offset)?)?;
    let l2_token_address = decode_string_field(take(L2_TOKEN_ADDRESS_LEN, &mut offset)?)?;

    if offset >= bytes.len() {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }
    let amount = decode_string_field(&bytes[offset..])?;

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
