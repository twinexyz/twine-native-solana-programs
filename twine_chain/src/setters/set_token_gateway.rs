use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{RoleType, TwineChainRoleManager},
    },
    utils::address_derivation::{derive_role_manager, verify_derived_address},
};

pub fn set_token_gateway(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_gateway_program: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_info_iter)?;
    let twine_operation_handler_acc = next_account_info(account_info_iter)?;

    // validate provided accounts
    let mut role_manager_data =
        validate_accounts(program_id, role_manager_acc, twine_operation_handler_acc)?;

    // Update Token Gateway value
    role_manager_data.token_gateway_program = token_gateway_program;

    role_manager_data
        .serialize(&mut &mut role_manager_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Token Gateway Updated");
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    role_manager_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
) -> Result<TwineChainRoleManager, ProgramError> {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate key
    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Deserialize account data
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Checks if signer has required role(TwineOperationHandler)
    if !role_manager_data.has_role(
        twine_operation_handler_acc.key,
        RoleType::TwineOperationHandler,
    ) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(role_manager_data)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES};
    use borsh::BorshDeserialize;
    use solana_program::{clock::Epoch, rent::Rent, system_program};
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
    fn test_set_token_gateway() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let twine_operation_handler_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut twine_operation_handler_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup Account Data (Gives role TwineOperationHandler to InitialChainAdmin)
        let role_manager_dummy_data = TwineChainRoleManager {
            is_initialized: true,
            chain_admin: Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
            twine_operator: Pubkey::default(),
            token_gateway_program: Pubkey::default(),
            roles: vec![(
                Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
                RoleType::TwineOperationHandler,
            )],
        };
        let mut role_manager_data = vec![];
        role_manager_dummy_data.serialize(&mut role_manager_data)?;

        let mut twine_operation_handler_data = vec![];
        let mut system_program_data = vec![];

        // Setup owners
        let mut role_manager_owner = program_id;
        let mut twine_operation_handler_owner = system_program_id;
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

        let twine_operation_handler_account = create_test_account_info(
            &twine_operation_handler_key,
            true,
            false,
            &mut twine_operation_handler_lamports,
            &mut twine_operation_handler_data,
            &mut twine_operation_handler_owner,
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
            twine_operation_handler_account.clone(),
            system_program_account.clone(),
        ];

        // call the set function
        let result = set_token_gateway(
            &program_id,
            &accounts,
            Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
        );
        assert!(result.is_ok(), "Setter failed: {:?}", result.err());

        // verify setter
        let role_manager_data =
            TwineChainRoleManager::deserialize(&mut &role_manager_account.data.borrow()[..])?;

        assert_eq!(
            role_manager_data.token_gateway_program,
            Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
            "The program should be set to provided pubkey"
        );
        Ok(())
    }
}
