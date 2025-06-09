use crate::core::error::ProgramCustomError;
use crate::core::state::{RoleType, TwineChainRoleManager, TwineChainStorage};
use crate::utils::address_derivation::{
    derive_role_manager, derive_twine_chain_storage, verify_derived_address,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn set_v_keys(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    groth16_vk: Vec<u8>,
    execution_vkey: String,
    inclusion_vkey: String,
    withdrawal_vkey: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let twine_operation_handler_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        twine_chain_storage_acc,
        twine_operation_handler_acc,
        role_manager_acc,
    )?;

    // Validate data length [2(0x) + 32Bytes(64 hex char) = 66 hex characters]
    if execution_vkey.len() > 66 || inclusion_vkey.len() > 66 || withdrawal_vkey.len() > 66 {
        return Err(ProgramCustomError::InvalidDataLength.into());
    }

    // Update account data
    let mut twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    twine_chain_storage_data.groth16_vk = groth16_vk;
    twine_chain_storage_data.execution_vkey = execution_vkey;
    twine_chain_storage_data.inclusion_vkey = inclusion_vkey;
    twine_chain_storage_data.withdrawal_vkey = withdrawal_vkey;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    twine_chain_storage_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_twine_chain_storage_pda, _twine_chain_storage_bump_seed) =
        derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Checks if signer has required role(TwineOperationHandler)
    let role_manager_data = TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(
        twine_operation_handler_acc.key,
        RoleType::TwineOperationHandler,
    ) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        core::state::BatchInfo,
        utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES},
    };
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
    fn test_set_v_keys() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (twine_chain_storage_key, _) = derive_twine_chain_storage(&program_id);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let twine_operation_handler_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let twine_chain_storage_space: usize = TwineChainStorage::LEN;
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut twine_chain_storage_lamports = rent.minimum_balance(twine_chain_storage_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut twine_operation_handler_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup Account Data
        let last_batch = BatchInfo {
            start_block: 0,
            end_block: 0,
        };

        let twine_chain_storage_data_dummy = TwineChainStorage {
            is_initialized: true,
            groth16_vk: Vec::new(),
            execution_vkey: String::from(
                "0x626c756500000000000000000000000000000000000000000000000000000111",
            ),
            inclusion_vkey: String::from(
                "0x626c756500000000000000000000000000000000000000000000000000000111",
            ),
            withdrawal_vkey: String::from(
                "0x626c756500000000000000000000000000000000000000000000000000000111",
            ),
            last_finalized_batch: last_batch.clone(),
            last_committed_batch: last_batch.clone(),
            last_transcation_finalized_batch: last_batch.clone(),
            last_finalized_receipt_root: [0u8; 32],
        };
        let mut twine_chain_storage_data = vec![];
        twine_chain_storage_data_dummy.serialize(&mut twine_chain_storage_data)?;
        let twine_chain_storage_space_value = twine_chain_storage_data.len(); // exact
        println!(
            "twine_chain_storage_space_value = {}",
            twine_chain_storage_space_value
        );

        // Gives role TwineOperationHandler to InitialChainAdmin
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
        let mut twine_chain_storage_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut twine_operation_handler_owner = system_program_id;
        let mut system_program_owner = system_program_id;

        // Create required account infos
        let twine_chain_storage_account = create_test_account_info(
            &twine_chain_storage_key,
            false,
            true,
            &mut twine_chain_storage_lamports,
            &mut twine_chain_storage_data,
            &mut twine_chain_storage_owner,
        );

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
            twine_chain_storage_account.clone(),
            twine_operation_handler_account.clone(),
            role_manager_account.clone(),
            system_program_account.clone(),
        ];

        // call the set function
        let bytes32_value =
            String::from("0x626c756500000000000000000000000000000000000000000000000000000000");
        let result = set_v_keys(
            &program_id,
            &accounts,
            Vec::new(),
            bytes32_value.clone(),
            bytes32_value.clone(),
            bytes32_value.clone(),
        );
        assert!(result.is_ok(), "Setter failed: {:?}", result.err());

        // verify setter
        let twine_chain_storage_data =
            TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data.borrow()[..])?;

        assert_eq!(
            twine_chain_storage_data.inclusion_vkey, bytes32_value,
            "Value should be set to value provided"
        );
        Ok(())
    }
}
