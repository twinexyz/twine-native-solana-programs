use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::clock::Clock;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
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
        state::{
            MessagesBuffer, MessagesReplicator, RoleType, TwineChainRoleManager, TwineChainStorage,
        },
    },
    utils::{
        address_derivation::{
            derive_messages_buffer, derive_messages_replicator, derive_role_manager,
            derive_twine_chain_storage, verify_derived_address, verify_system_program,
        },
        constants::{MAX_MESSAGE_NONCE, MEESSAGES_REPLICATOR_PREFIX, MESSAGE_NONCE_GAP},
    },
};

pub fn copy_messages_buffer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let (expected_meesage_pda, _) = derive_messages_buffer(program_id);
    verify_derived_address(expected_meesage_pda, messages_buffer_acc)?;
    
    // Deserialize account data
    let mut messages_buffer_data =
        MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !messages_buffer_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }
    
    if messages_buffer_data.message_nonce
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
        role_manager_acc,
        initializer_acc,
        system_program,
    )?;

    if messages_replicator_acc.data_is_empty() {
        let rent = Rent::default();
        let (_, deposit_messages_replicator_bump) =
            derive_messages_replicator(&program_id, start_nonce, end_nonce);
        let replicator_space = 1 + 8 + 8 + 8 + 4 + (MAX_MESSAGE_NONCE * 32);
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

    messages_replicator_data.start_nonce = start_nonce;
    messages_replicator_data.end_nonce = end_nonce;

    let messages_to_skip = messages_buffer_data.message_nonce - end_nonce;
    let end_index = messages_buffer_data
        .messages
        .len()
        .saturating_sub(messages_to_skip as usize);

    messages_replicator_data
        .messages
        .extend_from_slice(&messages_buffer_data.messages[..end_index]);

    messages_replicator_data
        .serialize(&mut &mut messages_replicator_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // Update Deposits
    messages_buffer_data.messages.drain(..end_index);

    messages_buffer_data
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
        
    let clock = Clock::get()?;
    msg!(
        "event=CopiedMessageBuffer start_nonce={}  end_nonce={}  slot_number={}",
        start_nonce,
        end_nonce,
        clock.slot
    );
    
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    start_nonce: u64,
    end_nonce: u64,
    messages_replicator_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);

    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    let (expected_messages_replicator_pda, _) =
        derive_messages_replicator(program_id, start_nonce, end_nonce);
    verify_derived_address(expected_messages_replicator_pda, messages_replicator_acc)?;

    // Checks if signer has required role(TwineOperationHandler)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(initializer_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    
    verify_system_program(system_program)?;

    Ok(())
}
