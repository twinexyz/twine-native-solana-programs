use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::json;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    rent::Rent,
    clock::Clock,
    sysvar::Sysvar,
    system_instruction,
};
use crate::{
    core::{
        error::ProgramCustomError,
        state::{BatchPdaAccount, RoleType, TwineChainRoleManager, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_commitment_pda, derive_role_manager, verify_derived_address,
            verify_system_program,derive_twine_chain_storage
        },
        constants::COMMITMENT_PDA_PREFIX,
    },
};

pub fn initialize_genesis_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    genesis_batch_hash: [u8; 32],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let first_batch_acc = next_account_info(account_iter)?;
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let initializer_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    let first_batch_space = BatchPdaAccount::LEN;

    let rent = Rent::default();

    // Validate Provided accounts
    let (genesis_batch_bump, mut twine_chain_storage_data) = validate_accounts(
        program_id,
        first_batch_acc,
        twine_chain_storage_acc,
        role_manager_acc,
        initializer_acc,
        system_program,
    )?;

    // Create Genesis Batch PDA
    let required_lamports = rent.minimum_balance(first_batch_space);
    let create_ix = system_instruction::create_account(
        initializer_acc.key,
        first_batch_acc.key,
        required_lamports,
        first_batch_space as u64,
        program_id,
    );
    invoke_signed(
        &create_ix,
        &[
            initializer_acc.clone(),
            first_batch_acc.clone(),
            system_program.clone(),
        ],
        &[&[
            COMMITMENT_PDA_PREFIX.as_bytes(),
            &0u64.to_be_bytes(),
            &[genesis_batch_bump],
        ]],
    )?;

    let mut account_data = BatchPdaAccount {
        is_initialized: true,
        batch_hash: [0u8; 32],
    };
    account_data.batch_hash = genesis_batch_hash;

    twine_chain_storage_data.last_committed_batch_number = 0;
    twine_chain_storage_data.last_committed_batch_hash = genesis_batch_hash;
    twine_chain_storage_data.last_finalized_batch_hash = genesis_batch_hash;

        // Update twine chain storage
    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    account_data
        .serialize(&mut &mut first_batch_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

     let clock = Clock::get()?;

    let event = json!(
        {
            "event": "GenesisBatchInitialized",
            "genesis_batch_hash": genesis_batch_hash,
            "slot_number": clock.slot
        }
    )
    .to_string();
    msg!(&event);
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    first_batch_acc: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<(u8, TwineChainStorage), ProgramError> {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;
    let (expected_commitment_pda, genesis_batch_bump) = derive_commitment_pda(&program_id, 0);
    verify_derived_address(expected_commitment_pda, first_batch_acc)?;
    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;
    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    verify_system_program(system_program)?;

    // re-initialization guard
    if !first_batch_acc.data_is_empty() {
        let first_batch_data =
            BatchPdaAccount::deserialize(&mut &first_batch_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;
        if first_batch_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    };

    // Checks if signer has required role(TwineOperationHandler)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(initializer_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok((genesis_batch_bump, twine_chain_storage_data))
}