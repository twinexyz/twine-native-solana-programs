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
        state::{
            DepositMessagesBuffer, DepositMessagesReplicator, RoleType, TwineChainRoleManager,
            TwineChainStorage,
        },
    },
    utils::{
        address_derivation::{
            derive_deposit_message_buffer, derive_deposit_messages_replicator, derive_role_manager,
            derive_twine_chain_storage, verify_derived_address, verify_system_program,
        },
        constants::{DEPOSIT_MEESSAGES_REPLICATOR_PREFIX, MAX_MESSAGE_NONCE, MESSAGE_NONCE_GAP},
    },
};

pub fn copy_deposit_pda(
    program_id: &Pubkey,
    start_nonce: u64,
    end_nonce: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let deposit_message_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let deposit_messages_replicator_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        start_nonce,
        end_nonce,
        deposit_message_buffer_acc,
        twine_chain_storage_acc,
        deposit_messages_replicator_acc,
        role_manager_acc,
        initializer_acc,
        system_program,
    )?;

    // Deserialize account data
    let mut deposits =
        DepositMessagesBuffer::deserialize(&mut &deposit_message_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !deposits.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if start_nonce != twine_chain_storage_data.last_copied_deposit_nonce + 1 {
        return Err(ProgramCustomError::InvalidStartNonce.into());
    }

    if (end_nonce - start_nonce) != MESSAGE_NONCE_GAP {
        return Err(ProgramCustomError::InvalidNonceGap.into());
    }

    if deposit_messages_replicator_acc.data_is_empty() {
        let rent = Rent::default();
        let (_, deposit_messages_replicator_bump) =
            derive_deposit_messages_replicator(&program_id, start_nonce, end_nonce);
        let replicator_space = 1 + 8 + 8 + 8 + 4 + (MAX_MESSAGE_NONCE * 32);
        let required_lamports = rent.minimum_balance(replicator_space);
        let create_ix = system_instruction::create_account(
            initializer_acc.key,
            deposit_messages_replicator_acc.key,
            required_lamports,
            replicator_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                initializer_acc.clone(),
                deposit_messages_replicator_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                DEPOSIT_MEESSAGES_REPLICATOR_PREFIX.as_bytes(),
                &start_nonce.to_be_bytes(),
                &end_nonce.to_be_bytes(),
                &[deposit_messages_replicator_bump],
            ]],
        )?;
    }

    let mut deposit_messages_replicator_data = DepositMessagesReplicator::deserialize(
        &mut &deposit_messages_replicator_acc.data.borrow()[..],
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    deposit_messages_replicator_data.start_nonce = start_nonce;
    deposit_messages_replicator_data.end_nonce = end_nonce;

    deposit_messages_replicator_data
        .deposit_messages
        .extend(deposits.deposit_messages.clone());

    deposit_messages_replicator_data
        .serialize(&mut &mut deposit_messages_replicator_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // Update Deposits
    deposits.deposit_messages.clear();

    deposits
        .serialize(&mut &mut deposit_message_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    start_nonce: u64,
    end_nonce: u64,
    deposit_message_buffer_acc: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    deposit_messages_replicator_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_deposit_pda, _) = derive_deposit_message_buffer(program_id);
    verify_derived_address(expected_deposit_pda, deposit_message_buffer_acc)?;

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);

    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    let (expected_deposit_messages_replicator_pda, _) =
        derive_deposit_messages_replicator(program_id, start_nonce, end_nonce);
    verify_derived_address(
        expected_deposit_messages_replicator_pda,
        deposit_messages_replicator_acc,
    )?;

    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

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
