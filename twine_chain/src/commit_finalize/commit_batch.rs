use borsh::{BorshDeserialize, BorshSerialize};
use serde_json;
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
        state::{
            BatchPdaAccount, CommitedBatchEvent, RoleType, TwineChainRoleManager, TwineChainStorage,
        },
    },
    utils::{
        address_derivation::{
            derive_commitment_pda, derive_role_manager, derive_twine_chain_storage,
            verify_derived_address, verify_system_program,
        },
        constants::{CHAIN_ID, COMMITMENT_PDA_PREFIX},
    },
};

pub fn commit_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    batch_number: u64,
    batch_hash: [u8; 32],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let current_batch_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let twine_operation_handler_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Validate PDAs
    let (current_pda_bump, mut twine_chain_storage_data) = validate_accounts(
        program_id,
        batch_number,
        twine_chain_storage_acc,
        current_batch_acc,
        role_manager_acc,
        twine_operation_handler_acc,
        system_program,
    )?;

    if batch_number != twine_chain_storage_data.last_committed_batch_number + 1 {
        return Err(ProgramCustomError::InvalidBlockCommitmentSequence.into());
    }

    // Initialize commitment PDA if not already initialized
    if current_batch_acc.data_is_empty() {
        let rent = Rent::default();
        let batch_space = BatchPdaAccount::LEN;
        let required_lamports = rent.minimum_balance(batch_space);
        let create_ix = system_instruction::create_account(
            twine_operation_handler_acc.key,
            current_batch_acc.key,
            required_lamports,
            batch_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                twine_operation_handler_acc.clone(),
                current_batch_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                COMMITMENT_PDA_PREFIX.as_bytes(),
                &batch_number.to_be_bytes(),
                &[current_pda_bump],
            ]],
        )?;
        let batch_data = BatchPdaAccount {
            is_initialized: true,
            batch_hash: [0u8; 32],
        };

        batch_data
            .serialize(&mut &mut current_batch_acc.data.borrow_mut()[..])
            .map_err(|_| ProgramCustomError::SerializeFailed)?;
    }

    let mut current_batch_data =
        BatchPdaAccount::deserialize(&mut &current_batch_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !current_batch_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    current_batch_data.batch_hash = batch_hash;

    // Update current batch
    current_batch_data
        .serialize(&mut &mut current_batch_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    twine_chain_storage_data.last_committed_batch_number = batch_number;
    twine_chain_storage_data.last_committed_batch_hash = batch_hash;

    // Update twine chain storage
    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // Emit event
    let clock = Clock::get()?;

    let event = CommitedBatchEvent {
        event: "CommitedBatch".to_string(),
        batch_number: batch_number,
        chain_id: CHAIN_ID,
        batch_hash: batch_hash,
        slot_number: clock.slot,
    };

    let serialized_event =
        serde_json::to_string(&event).map_err(|_| ProgramError::InvalidInstructionData)?;
    msg!("{}", serialized_event);

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    batch_number: u64,
    twine_chain_storage_acc: &AccountInfo,
    current_batch_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<(u8, TwineChainStorage), ProgramError> {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Dervie and validate PDAs
    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    let (expected_current_pda, current_pda_bump) = derive_commitment_pda(program_id, batch_number);
    verify_derived_address(expected_current_pda, current_batch_acc)?;

    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if initiator has TwineOperationHandler Role
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(
        twine_operation_handler_acc.key,
        RoleType::TwineOperationHandler,
    ) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    verify_system_program(system_program)?;

    Ok((current_pda_bump, twine_chain_storage_data))
}
