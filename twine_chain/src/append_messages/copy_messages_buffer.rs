use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::json;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{DetailedMessagesBuffer, MessagesReplicator, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_messages_replicator,
            derive_twine_chain_storage, verify_derived_address, verify_system_program,
        },
        constants::{MEESSAGES_REPLICATOR_PREFIX, MESSAGE_NONCE_GAP, MESSAGE_NONCE_GAP_SIZE},
    },
};

pub fn copy_messages_buffer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let mut twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let (expected_detailed_messages_pda, _) = derive_detailed_messages_buffer(program_id);
    verify_derived_address(expected_detailed_messages_pda, detailed_messages_buffer_acc)?;

    // Deserialize account data
    let mut detailed_messages_buffer_data =
        DetailedMessagesBuffer::deserialize(&mut &detailed_messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !detailed_messages_buffer_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    if detailed_messages_buffer_data.message_nonce
        < twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP
    {
        return Err(ProgramCustomError::InvalidNonceGap.into());
    }

    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;

    validate_accounts(
        program_id,
        start_nonce,
        end_nonce,
        messages_replicator_acc,
        initializer_acc,
        system_program,
    )?;

    if messages_replicator_acc.data_is_empty() {
        let rent = Rent::default();

        let (_, deposit_messages_replicator_bump) =
            derive_messages_replicator(program_id, start_nonce, end_nonce);
        let replicator_space = 1 + 8 + 8 + 8 + 4 + (MESSAGE_NONCE_GAP_SIZE * 32);
        let required_lamports = rent.minimum_balance(replicator_space);
        let create_ix = system_instruction::create_account(
            initializer_acc.key,
            messages_replicator_acc.key,
            required_lamports,
            replicator_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                initializer_acc.clone(),
                messages_replicator_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                MEESSAGES_REPLICATOR_PREFIX.as_bytes(),
                &start_nonce.to_be_bytes(),
                &end_nonce.to_be_bytes(),
                &[deposit_messages_replicator_bump],
            ]],
        )?;
    }

    let mut messages_replicator_data =
        MessagesReplicator::deserialize(&mut &messages_replicator_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    messages_replicator_data.is_initialized = true;
    messages_replicator_data.start_nonce = start_nonce;
    messages_replicator_data.end_nonce = end_nonce;

    let messages_to_skip = detailed_messages_buffer_data.message_nonce - end_nonce;

    let end_index = detailed_messages_buffer_data
        .messages
        .len()
        .saturating_sub(messages_to_skip as usize);

    messages_replicator_data
        .messages
        .extend_from_slice(&detailed_messages_buffer_data.messages[..end_index]);

    messages_replicator_data
        .serialize(&mut &mut messages_replicator_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // Update Deposits
    let messages_to_keep = detailed_messages_buffer_data.messages.split_off(end_index);
    detailed_messages_buffer_data.messages = messages_to_keep;

    detailed_messages_buffer_data
        .serialize(&mut &mut detailed_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    twine_chain_storage_data.last_copied_message_start_nonce = start_nonce;
    twine_chain_storage_data.last_copied_message_end_nonce = end_nonce;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let clock = Clock::get()?;

    let event = json!({
        "event": "CopiedMessageBuffer",
        "start_nonce": start_nonce,
        "end_nonce": end_nonce,
        "slot_number": clock.slot,
    })
    .to_string();

    msg!(&event);

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    start_nonce: u64,
    end_nonce: u64,
    messages_replicator_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_messages_replicator_pda, _) =
        derive_messages_replicator(program_id, start_nonce, end_nonce);
    verify_derived_address(expected_messages_replicator_pda, messages_replicator_acc)?;

    verify_system_program(system_program)?;

    Ok(())
}
