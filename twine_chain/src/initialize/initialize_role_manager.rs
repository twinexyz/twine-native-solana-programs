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
        state::{RoleType, TwineChainRoleManager},
    },
    utils::{
        address_derivation::{
            derive_twine_chain_role_manager, verify_derived_address, verify_system_program,
        },
        constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES, ROLE_MANAGER_PREFIX},
    },
};

pub fn initialize_role_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    let rent = Rent::default();

    // Dervive and validate PDA
    let role_manager_bump = validate_accounts(
        program_id,
        role_manager_acc,
        chain_admin_acc,
        system_program,
    )?;

    if role_manager_acc.data_is_empty() {
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);
        let required_lamports = rent.minimum_balance(role_manager_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            role_manager_acc.key,
            required_lamports,
            role_manager_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                role_manager_acc.clone(),
                system_program.clone(),
            ],
            &[&[ROLE_MANAGER_PREFIX.as_bytes(), &[role_manager_bump]]],
        )?;
    }
    let chain_admin: Pubkey = INITIAL_CHAIN_ADMIN.parse().expect("Invalid Pubkey");
    let role_manager_data = TwineChainRoleManager {
        is_initialized: true,
        chain_admin: chain_admin,
        roles: vec![(chain_admin, RoleType::TwineOperationHandler)],
    };
    role_manager_data
        .serialize(&mut &mut role_manager_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Twine Chain Role Manager Initialized");
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<u8, ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_role_manager_pda, role_manager_bump) =
        derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    verify_system_program(system_program)?;

    if !role_manager_acc.data_is_empty() {
        let role_manager_data =
            TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;

        if role_manager_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    Ok(role_manager_bump)
}
