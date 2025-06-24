use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::program::invoke_signed;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use crate::{
    core::state::{RoleType, TokensGatewayRoleManager},
    utils::{
        address_derivation::derive_gateway_role_manager,
        constants::{INITIAL_CHAIN_ADMIN, ROLE_MANAGER_ACCOUNT_SIZE, ROLE_MANAGER_PREFIX},
    },
};

pub fn initialize_role_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    validate_accounts(
        role_manager_acc,
        chain_admin_acc,
        system_program,
        program_id,
    )?;
    msg!("2");
    let rent = Rent::default();
    let (_, role_bump) = derive_gateway_role_manager(&program_id);

    let required_lamports = rent.minimum_balance(ROLE_MANAGER_ACCOUNT_SIZE);
    let create_ix = system_instruction::create_account(
        chain_admin_acc.key,
        role_manager_acc.key,
        required_lamports,
        ROLE_MANAGER_ACCOUNT_SIZE as u64,
        program_id,
    );
    invoke_signed(
        &create_ix,
        &[
            chain_admin_acc.clone(),
            role_manager_acc.clone(),
            system_program.clone(),
        ],
        &[&[ROLE_MANAGER_PREFIX.as_bytes(), &[role_bump]]],
    )?;

    let chain_admin: Pubkey = INITIAL_CHAIN_ADMIN
        .parse()
        .map_err(|_| ProgramError::InvalidArgument)?;

    let role_manager = TokensGatewayRoleManager {
        is_initialized: true,
        chain_admin,
        roles: vec![(chain_admin, RoleType::TwineOperationHandler)],
    };

    let mut data = role_manager_acc.data.borrow_mut();
    role_manager.serialize(&mut &mut data[..])?;

    msg!(
        "EVENT:RoleManagerInitialized: role_manager={}, chain_admin={}, initial_role={}",
        role_manager_acc.key,
        chain_admin,
        "TwineOperationHandler"
    );

    Ok(())
}

fn validate_accounts(
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
      msg!("1");
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.key != &solana_program::system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check if account is already initialized
    {
        let data = role_manager_acc.data.borrow();
        if !data.is_empty() {
            let mut data_slice = &data[..];
            if let Ok(role_manager) = TokensGatewayRoleManager::deserialize(&mut data_slice) {
                if role_manager.is_initialized {
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
            }
        }
    }

    let (expected_role_manager_key, _) = derive_gateway_role_manager(&program_id);
    msg!("Expected Role Manager: {}",expected_role_manager_key);
    msg!("sent role Manager: {}",*role_manager_acc.key);
    if expected_role_manager_key != *role_manager_acc.key {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

#[cfg(test)]
fn invoke_signed(
    _instruction: &solana_program::instruction::Instruction,
    account_infos: &[solana_program::account_info::AccountInfo],
    _signer_seeds: &[&[&[u8]]],
) -> solana_program::entrypoint::ProgramResult {
    let role_manager_acc = &account_infos[1]; // role_manager_acc is second
    let mut data = role_manager_acc.data.borrow_mut();

    // Initialize with empty data
    data.fill(0);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::state::TokensGatewayRoleManager;
    use solana_program::{
        account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, rent::Rent, system_program,
    };
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
    fn setup_test_accounts(
        program_id: &Pubkey,
    ) -> (
        Pubkey,
        Pubkey,
        Pubkey,
        u64,
        u64,
        u64,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Pubkey,
        Pubkey,
        Pubkey,
    ) {
        let (role_manager_key, _bump) =
            Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
        let chain_admin_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN).unwrap();
        let system_program_id = system_program::id();

        let rent = Rent::default();
        let required_lamports = rent.minimum_balance(ROLE_MANAGER_ACCOUNT_SIZE);

        let role_manager_lamports = required_lamports;
        let chain_admin_lamports = 1_000_000_000;
        let system_program_lamports = 0;

        let role_manager_data = vec![0u8; ROLE_MANAGER_ACCOUNT_SIZE];
        let chain_admin_data = vec![];
        let system_program_data = vec![];

        let role_manager_owner = *program_id;
        let chain_admin_owner = system_program_id;
        let system_program_owner = system_program_id;

        (
            role_manager_key,
            chain_admin_key,
            system_program_id,
            role_manager_lamports,
            chain_admin_lamports,
            system_program_lamports,
            role_manager_data,
            chain_admin_data,
            system_program_data,
            role_manager_owner,
            chain_admin_owner,
            system_program_owner,
        )
    }

    #[test]
    fn test_initialize_role_manager_success() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();
        let (role_manager_key, _bump) =
            Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], &program_id);
        let chain_admin_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;

        let rent = Rent::default();
        let required_lamports = rent.minimum_balance(ROLE_MANAGER_ACCOUNT_SIZE);

        let mut role_manager_lamports = required_lamports;
        let mut role_manager_data = vec![0u8; ROLE_MANAGER_ACCOUNT_SIZE];
        let mut role_manager_owner = program_id;

        let mut chain_admin_lamports = 1_000_000_000;
        let mut chain_admin_data = vec![];
        let mut chain_admin_owner = system_program::id();

        let mut system_program_lamports = 0;
        let mut system_program_data = vec![];
        let mut system_program_owner = system_program::id();

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

        let system_program_id = system_program::id();
        let system_program_account = create_test_account_info(
            &system_program_id,
            false,
            false,
            &mut system_program_lamports,
            &mut system_program_data,
            &mut system_program_owner,
        );

        let accounts = vec![
            role_manager_account.clone(),
            chain_admin_account,
            system_program_account,
        ];

        let result = initialize_role_manager(&program_id, &accounts);
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        let data_ref = role_manager_account.data.borrow();
        let mut data_slice = &data_ref[..];
        let deserialized = TokensGatewayRoleManager::deserialize(&mut data_slice)?;

        assert!(deserialized.is_initialized, "Account should be initialized");
        assert_eq!(
            deserialized.chain_admin, chain_admin_key,
            "Chain admin should be set"
        );
        assert_eq!(
            deserialized.roles[0],
            (chain_admin_key, RoleType::TwineOperationHandler),
            "Role should be assigned to chain_admin"
        );

        Ok(())
    }
    #[test]
    fn test_initialize_role_manager_already_initialized() -> Result<(), Box<dyn std::error::Error>>
    {
        let program_id = Pubkey::new_unique();
        let (
            role_manager_key,
            chain_admin_key,
            system_program_id,
            mut role_manager_lamports,
            mut chain_admin_lamports,
            mut system_program_lamports,
            mut role_manager_data,
            mut chain_admin_data,
            mut system_program_data,
            mut role_manager_owner,
            mut chain_admin_owner,
            mut system_program_owner,
        ) = setup_test_accounts(&program_id);

        // Pre-initialize the account data
        let existing_role_manager = TokensGatewayRoleManager {
            is_initialized: true,
            chain_admin: chain_admin_key,
            roles: vec![],
        };
        let serialized_data = existing_role_manager.try_to_vec()?;
        role_manager_data[..serialized_data.len()].copy_from_slice(&serialized_data);

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

        let accounts = vec![
            role_manager_account,
            chain_admin_account,
            system_program_account,
        ];

        let result = initialize_role_manager(&program_id, &accounts);
        assert!(
            result.is_err(),
            "Should fail when account is already initialized"
        );
        assert_eq!(
            result.unwrap_err(),
            ProgramError::AccountAlreadyInitialized,
            "Should return AccountAlreadyInitialized error"
        );

        Ok(())
    }
    #[test]
    fn test_initialize_role_manager_missing_signature() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();
        let (
            role_manager_key,
            chain_admin_key,
            system_program_id,
            mut role_manager_lamports,
            mut chain_admin_lamports,
            mut system_program_lamports,
            mut role_manager_data,
            mut chain_admin_data,
            mut system_program_data,
            mut role_manager_owner,
            mut chain_admin_owner,
            mut system_program_owner,
        ) = setup_test_accounts(&program_id);

        let role_manager_account = create_test_account_info(
            &role_manager_key,
            false,
            true,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        // Chain admin is NOT a signer
        let chain_admin_account = create_test_account_info(
            &chain_admin_key,
            false, // Not a signer
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

        let accounts = vec![
            role_manager_account,
            chain_admin_account,
            system_program_account,
        ];

        let result = initialize_role_manager(&program_id, &accounts);
        assert!(
            result.is_err(),
            "Should fail when chain admin is not a signer"
        );
        assert_eq!(
            result.unwrap_err(),
            ProgramError::MissingRequiredSignature,
            "Should return MissingRequiredSignature error"
        );

        Ok(())
    }
}
