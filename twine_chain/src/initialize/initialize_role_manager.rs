use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::program::invoke_signed;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
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
        address_derivation::{derive_role_manager, verify_derived_address, verify_system_program},
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
        twine_operator: Pubkey::default(),
        token_gateway_program: Pubkey::default(),
        roles: vec![
            (chain_admin, RoleType::TwineOperationHandler),
            (chain_admin, RoleType::MessageAppender),
        ],
    };

    role_manager_data
        .serialize(&mut &mut role_manager_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Role Manager Initialized");
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

    let (expected_role_manager_pda, role_manager_bump) = derive_role_manager(program_id);
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

#[cfg(test)]
fn invoke_signed(
    _ix: &solana_program::instruction::Instruction,
    account_infos: &[solana_program::account_info::AccountInfo],
    _signer_seeds: &[&[&[u8]]],
) -> solana_program::entrypoint::ProgramResult {
    use std::mem;

    for acc in account_infos.iter() {
        if !acc.is_writable {
            continue;
        }
        // For testing purpose, allocate a large space to every PDA
        let space = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);
        let leaked: &'static mut [u8] = Box::leak(vec![0u8; space].into_boxed_slice());
        unsafe {
            let mut data_ref = acc.data.borrow_mut();
            *data_ref = mem::transmute::<&'static mut [u8], &mut [u8]>(leaked);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::constants::INITIAL_CHAIN_ADMIN;
    use solana_program::{clock::Epoch, system_program};
    use std::str::FromStr;

    fn create_test_account_info<'a>(
        key: &'a Pubkey,
        is_signer: bool,
        is_writable: bool,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        owner: &'a mut Pubkey,
    ) -> AccountInfo<'a> {
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            false,
            Epoch::default(),
        )
    }

    #[test]
    fn test_role_manager_initialization() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let chain_admin_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut chain_admin_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup Account Data
        let mut role_manager_data = vec![];
        let mut chain_admin_data = vec![];
        let mut system_program_data = vec![];

        // Setup owners
        let mut role_manager_owner = program_id;
        let mut chain_admin_owner = system_program_id;
        let mut system_program_owner = system_program_id;

        // Create required account infos

        let role_manager_account = create_test_account_info(
            &role_manager_key,
            false,
            true,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        let chain_admin_account = create_test_account_info(
            &chain_admin_key,
            true,
            false,
            &mut chain_admin_lamports,
            &mut chain_admin_data,
            &mut chain_admin_owner,
        );

        let system_program_account = create_test_account_info(
            &system_program_id,
            false,
            false,
            &mut system_program_lamports,
            &mut system_program_data,
            &mut system_program_owner,
        );

        // Create accounts array in the correct order matching the function
        let accounts = vec![
            role_manager_account.clone(),
            chain_admin_account.clone(),
            system_program_account.clone(),
        ];

        // Call the initialize function
        let result = initialize_role_manager(&program_id, &accounts);
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        // verify role manager initialization
        let role_manager_data =
            TwineChainRoleManager::deserialize(&mut &role_manager_account.data.borrow()[..])?;

        assert!(
            role_manager_data.is_initialized(),
            "Genesis batch should be initialized"
        );

        assert_eq!(
            role_manager_data.chain_admin,
            INITIAL_CHAIN_ADMIN.parse()?,
            "The chain admin should match with the one provided"
        );

        Ok(())
    }
}
