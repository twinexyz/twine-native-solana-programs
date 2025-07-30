use borsh::{BorshDeserialize, BorshSerialize};
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
// #[cfg(not(test))]
use solana_program::program::invoke_signed;

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
        twine_chain_storage_acc,
        first_batch_acc,
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

    msg!("Genesis Batch Initialized");

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    twine_chain_storage_acc: &AccountInfo,
    first_batch_acc: &AccountInfo,
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

// #[cfg(test)]
// fn invoke_signed(
//     _ix: &solana_program::instruction::Instruction,
//     account_infos: &[solana_program::account_info::AccountInfo],
//     _signer_seeds: &[&[&[u8]]],
// ) -> solana_program::entrypoint::ProgramResult {
//     use std::mem;

//     for acc in account_infos.iter() {
//         if !acc.is_writable {
//             continue;
//         }
//         // For testing purpose, allocate a large space to every PDA
//         let space = 1 + (4 + BlockInfo::LEN) + 1 + 1;
//         let leaked: &'static mut [u8] = Box::leak(vec![0u8; space].into_boxed_slice());
//         unsafe {
//             let mut data_ref = acc.data.borrow_mut();
//             *data_ref = mem::transmute::<&'static mut [u8], &mut [u8]>(leaked);
//         }
//     }
//     Ok(())
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES};
//     use borsh::BorshDeserialize;
//     use solana_program::{clock::Epoch, system_program};
//     use std::str::FromStr;

//     fn create_test_account_info<'a>(
//         key: &'a Pubkey,
//         is_signer: bool,
//         is_writable: bool,
//         lamports: &'a mut u64,
//         data: &'a mut [u8],
//         owner: &'a mut Pubkey,
//     ) -> AccountInfo<'a> {
//         AccountInfo::new(
//             key,
//             is_signer,
//             is_writable,
//             lamports,
//             data,
//             owner,
//             false,
//             Epoch::default(),
//         )
//     }

//     #[test]
//     fn test_genesis_batch_initialization() -> Result<(), Box<dyn std::error::Error>> {
//         let program_id = Pubkey::new_unique();

//         // Get the required accounts
//         let (first_batch_key, _) = derive_commitment_pda(&program_id, 0u64, 0u64);
//         let (role_manager_key, _) = derive_role_manager(&program_id);
//         let initializer_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
//         let system_program_id = system_program::id();

//         // Required space for each account
//         let first_batch_space = 1 + (4 + BlockInfo::LEN) + 1 + 1;
//         let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

//         // Setup Account Lamports
//         let rent = Rent::default();
//         let mut first_batch_lamports = rent.minimum_balance(first_batch_space);
//         let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
//         let mut initializer_lamports = 1_000_000_000;
//         let mut system_program_lamports = 0;

//         // Setup Account Data
//         let mut first_batch_data = vec![];
//         let mut initializer_data = vec![];
//         let mut system_program_data = vec![];

//         // Give Role twineOperatioHandler to InitialChainAdmin
//         let role_manager_dummy_data = TwineChainRoleManager {
//             is_initialized: true,
//             chain_admin: Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
//             twine_operator: Pubkey::default(),
//             token_gateway_program: Pubkey::default(),
//             roles: vec![(
//                 Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
//                 RoleType::TwineOperationHandler,
//             )],
//         };
//         let mut role_manager_data = vec![];
//         role_manager_dummy_data.serialize(&mut role_manager_data)?;

//         // Setup owners
//         let mut first_batch_owner = program_id;
//         let mut role_manager_owner = program_id;
//         let mut initializer_owner = system_program_id;
//         let mut system_program_owner = system_program_id;

//         // Create required account infos
//         let first_batch_account = create_test_account_info(
//             &first_batch_key,
//             false,
//             true,
//             &mut first_batch_lamports,
//             &mut first_batch_data,
//             &mut first_batch_owner,
//         );

//         let role_manager_account = create_test_account_info(
//             &role_manager_key,
//             false,
//             true,
//             &mut role_manager_lamports,
//             &mut role_manager_data,
//             &mut role_manager_owner,
//         );

//         let initializer_account = create_test_account_info(
//             &initializer_key,
//             true,
//             false,
//             &mut initializer_lamports,
//             &mut initializer_data,
//             &mut initializer_owner,
//         );

//         let system_program_account = create_test_account_info(
//             &system_program_id,
//             false,
//             false,
//             &mut system_program_lamports,
//             &mut system_program_data,
//             &mut system_program_owner,
//         );

//         // Create accounts array in the correct order matching the function
//         let accounts = vec![
//             first_batch_account.clone(),
//             role_manager_account.clone(),
//             initializer_account.clone(),
//             system_program_account.clone(),
//         ];

//         // Call the initialize function
//         let result = initialize_genesis_batch(&program_id, &accounts, [1u8; 32]);
//         assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

//         // verify genesis batch initialization
//         let genesis_batch_data =
//             BatchPdaAccount::deserialize(&mut &first_batch_account.data.borrow()[..])?;

//         assert!(
//             genesis_batch_data.is_initialized(),
//             "Genesis batch should be initialized"
//         );

//         assert!(
//             genesis_batch_data.verified,
//             "Genesis batch should be verified"
//         );

//         assert!(genesis_batch_data.is_full, "Genesis batch should be full");

//         assert_eq!(
//             genesis_batch_data.infos.len(),
//             1,
//             "There should be one batch data"
//         );

//         assert_eq!(
//             genesis_batch_data.infos[0].block_hash, [1u8; 32],
//             "Block hash should be same as the one provided"
//         );
//         Ok(())
//     }
// }
