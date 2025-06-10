use crate::core::error::ProgramCustomError;
use crate::core::state::{BatchInfo, TwineChainRoleManager, TwineChainStorage};
use crate::utils::address_derivation::{
    derive_role_manager, derive_twine_chain_storage, verify_derived_address, verify_owner,
    verify_system_program,
};
use crate::utils::constants::TWINE_CHAIN_STORAGE_PREFIX;
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

pub fn initialize_chain_storage(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Dervive and validate PDA
    let twine_chain_storage_bump = validate_accounts(
        program_id,
        twine_chain_storage_acc,
        role_manager_acc,
        chain_admin_acc,
        system_program,
    )?;

    let rent = Rent::default();

    // Create account
    if twine_chain_storage_acc.data_is_empty() {
        let chain_storage_space = TwineChainStorage::LEN;
        let required_lamports = rent.minimum_balance(chain_storage_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            twine_chain_storage_acc.key,
            required_lamports,
            chain_storage_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                twine_chain_storage_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                TWINE_CHAIN_STORAGE_PREFIX.as_bytes(),
                &[twine_chain_storage_bump],
            ]],
        )?;
    }

    // Update the data
    let last_batch = BatchInfo {
        start_block: 0,
        end_block: 0,
    };

    let twine_chain_storage_data = TwineChainStorage {
        is_initialized: true,
        groth16_vk: Vec::new(),
        execution_vkey: String::from(""),
        inclusion_vkey: String::from(""),
        withdrawal_vkey: String::from(""),
        skip_verification: false,
        last_finalized_batch: last_batch.clone(),
        last_committed_batch: last_batch.clone(),
        last_transaction_finalized_batch: last_batch.clone(),
        last_finalized_receipt_root: [0u8; 32],
    };

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Role Manager Initialized");

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    twine_chain_storage_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<u8, ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_twine_chain_storage_pda, twine_chain_storage_bump) =
        derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;
    verify_owner(twine_chain_storage_acc, program_id)?;

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;
    verify_owner(role_manager_acc, program_id)?;

    verify_system_program(system_program)?;

    if !twine_chain_storage_acc.data_is_empty() {
        let twine_chain_data: TwineChainStorage =
            TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;

        if twine_chain_data.is_initialized() {
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

    Ok(twine_chain_storage_bump)
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
        let space = TwineChainStorage::LEN;
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
    use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES};
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
    fn test_twine_chain_storage_initialization() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (twine_chain_storage_acc, _) = derive_twine_chain_storage(&program_id);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let chain_admin_acc = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let twine_chain_storage_space = TwineChainStorage::LEN;
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut twine_chain_storage_lamports = rent.minimum_balance(twine_chain_storage_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut chain_admin_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup Account data
        let mut twine_chain_storage_data = vec![];
        let mut chain_admin_data = vec![];
        let mut system_program_data = vec![];

        // Set InitialChainAdmin as chain admin
        let role_manager_dummy_data = TwineChainRoleManager {
            is_initialized: true,
            chain_admin: Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
            twine_operator: Pubkey::default(),
            token_gateway_program: Pubkey::default(),
            roles: vec![],
        };
        let mut role_manager_data = vec![];
        role_manager_dummy_data.serialize(&mut role_manager_data)?;

        // Setup owners
        let mut twine_chain_storage_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut chain_admin_owner = system_program_id;
        let mut system_program_owner = system_program_id;

        // Create required account infos
        let twine_chain_storage_account = create_test_account_info(
            &twine_chain_storage_acc,
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

        let initializer_account = create_test_account_info(
            &chain_admin_acc,
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
            twine_chain_storage_account.clone(),
            role_manager_account.clone(),
            initializer_account.clone(),
            system_program_account.clone(),
        ];

        // Call the initialize function
        let result = initialize_chain_storage(&program_id, &accounts);
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        // verify Twine chain storage initialization
        let twine_chain_storage_data =
            TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data.borrow()[..])?;

        assert!(
            twine_chain_storage_data.is_initialized(),
            "Twine chain storage should be initialized"
        );

        assert_eq!(
            twine_chain_storage_data.last_finalized_receipt_root, [0u8; 32],
            "There receipt root should be 0 bytes32"
        );

        assert_eq!(
            twine_chain_storage_data.last_committed_batch.start_block, 0,
            "last committed batch's start block should be 0"
        );

        Ok(())
    }
}
