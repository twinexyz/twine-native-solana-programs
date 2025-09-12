use borsh::{BorshDeserialize, BorshSerialize};
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
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{DetailedMessagesBuffer, MessagesBuffer, TwineChainRoleManager},
    },
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_messages_buffer,
            derive_twine_chain_role_manager, verify_derived_address, verify_owner,
            verify_system_program,
        },
        constants::{CHAIN_ID, DETAILED_MESSAGES_BUFFER_PREFIX,MESSAGE_NONCE_GAP_SIZE, MESSAGES_BUFFER_PREFIX},
    },
};

pub fn initialize_message_buffer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Validate Provided accounts
    let (messages_bump, detailed_messages_bump) = validate_accounts(
        program_id,
        messages_buffer_acc,
        detailed_messages_buffer_acc,
        role_manager_acc,
        chain_admin_acc,
        system_program,
    )?;
    let rent = Rent::default();

    /**************************
     *  Message Buffer *
     *************************/
    if messages_buffer_acc.data_is_empty() {
        let messages_buffer_space = 10240;
        let required_lamports = rent.minimum_balance(messages_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            messages_buffer_acc.key,
            required_lamports,
            messages_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[MESSAGES_BUFFER_PREFIX.as_bytes(), &[messages_bump]]],
        )?;
    }

    // Update account data
    let messages_buffer_data = MessagesBuffer {
        is_initialized: true,
        message_nonce: 0,
        chain_id: CHAIN_ID,
        messages_rolling_hash: [0u8; 32],
    };

    // Serialize account data
    messages_buffer_data
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Message Buffer Initialized");
    /**************************
     *  Detailed Message Buffer *
     *************************/
    if detailed_messages_buffer_acc.data_is_empty() {
        let detailed_messages_buffer_space = 1 + 8 + 8 + 8 + 4 + (MESSAGE_NONCE_GAP_SIZE * 32);
        let required_lamports = rent.minimum_balance(detailed_messages_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            detailed_messages_buffer_acc.key,
            required_lamports,
            detailed_messages_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                detailed_messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                DETAILED_MESSAGES_BUFFER_PREFIX.as_bytes(),
                &[detailed_messages_bump],
            ]],
        )?;
    }

    // Update account data
    let detailed_messages_buffer_data = DetailedMessagesBuffer {
        is_initialized: true,
        message_nonce: 0,
        chain_id: CHAIN_ID,
        messages: Vec::new(),
    };

    // Serialize account data
    detailed_messages_buffer_data
        .serialize(&mut &mut detailed_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Detailed Message Buffer Initialized");

    Ok(())
}

// TODO: Check if chain_admin_acc has required role(Chain Admin)
fn validate_accounts(
    program_id: &Pubkey,
    messages_buffer_acc: &AccountInfo,
    detailed_messages_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<(u8, u8), ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate Account key and owner
    let (expected_role_manager_pda, _) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;
    verify_owner(role_manager_acc, program_id)?;

    let (expected_messages_pda, messages_bump) = derive_messages_buffer(program_id);
    verify_derived_address(expected_messages_pda, messages_buffer_acc)?;

    let (expected_detailed_messages_pda, detailed_messages_bump) =
        derive_detailed_messages_buffer(program_id);
    verify_derived_address(expected_detailed_messages_pda, detailed_messages_buffer_acc)?;

    verify_system_program(system_program)?;

    // re-initialization guard for deposit buffer
    if !messages_buffer_acc.data_is_empty() {
        let deposit_buffer_data =
            MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;
        if deposit_buffer_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // Checks if signer has required role(ChainAdmin)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if role_manager_data.chain_admin != *chain_admin_acc.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok((messages_bump, detailed_messages_bump))
}
