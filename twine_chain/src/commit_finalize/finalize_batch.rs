use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
#[cfg(not(test))]
use solana_program::clock::Clock;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{BatchPdaAccount, RoleType, TwineChainRoleManager, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_commitment_pda, derive_role_manager, derive_twine_chain_storage,
            verify_derived_address,
        },
        constants::CHAIN_ID,
    },
};

pub fn finalize_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    batch_number: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let current_batch_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let twine_operation_handler_acc = next_account_info(account_iter)?;

    let (executed_message_count, previous_batch_hash, current_batch_hash) =
        decode_batch_info(&public_values)?;

    let mut twine_chain_storage_data = validate_pdas(
        program_id,
        batch_number,
        previous_batch_hash,
        current_batch_hash,
        twine_chain_storage_acc,
        current_batch_acc,
        role_manager_acc,
        twine_operation_handler_acc,
    )?;

    // Checking if batch is filled
    let current_batch_data =
        BatchPdaAccount::deserialize(&mut &current_batch_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !current_batch_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    };

    if current_batch_data.batch_hash != current_batch_hash {
        return Err(ProgramCustomError::BatchHashMismatch.into());
    };

    if executed_message_count < twine_chain_storage_data.total_msg_handled_on_twine {
        return Err(ProgramCustomError::MessageExecutedCountError.into());
    }

    // Calling SP1 Verifier to verify the execution proof
    if !twine_chain_storage_data.skip_verification {
        verify_proof(
            &execution_proof,
            &public_values,
            &twine_chain_storage_data.execution_vkey,
            GROTH16_VK_4_0_0_RC3_BYTES,
        )
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    }

    // Updating the states
    twine_chain_storage_data.last_finalized_batch_number = batch_number;
    twine_chain_storage_data.last_finalized_batch_hash = current_batch_hash;

    current_batch_data
        .serialize(&mut &mut current_batch_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let clock = Clock::get()?;
    msg!(
        "event=BatchFinalizationSuccessful batch_number={}  chain_id={} batch_hash={:?} slot_number={}",
        batch_number,
        CHAIN_ID,
        current_batch_hash,
        clock.slot
    );
    Ok(())
}

pub fn decode_batch_info(bytes: &[u8]) -> Result<(u64, [u8; 32], [u8; 32]), ProgramError> {
    const LEN: usize = 32 + 32 + 8 + 8; 

    if bytes.len() != LEN {
        return Err(ProgramCustomError::PublicValueDecodeFailed.into());
    }

    let solana_message_count = u64::from_be_bytes(bytes[72..80].try_into().unwrap());
    let mut prev = [0u8; 32];
    prev.copy_from_slice(&bytes[0..32]);
    let mut curr = [0u8; 32];
    curr.copy_from_slice(&bytes[32..64]);

    Ok((solana_message_count, prev, curr))
}

fn validate_pdas(
    program_id: &Pubkey,
    batch_number: u64,
    previous_batch_hash: [u8; 32],
    current_batch_hash: [u8; 32],
    twine_chain_storage_acc: &AccountInfo,
    current_batch_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
) -> Result<TwineChainStorage, ProgramError> {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // Dervie and validate PDAs
    let (expected_twine_chain_storage_pda, _storage_bump) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    let (expected_current_pda, _current_pda_bump) = derive_commitment_pda(program_id, batch_number);
    verify_derived_address(expected_current_pda, current_batch_acc)?;
    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if batch_number != twine_chain_storage_data.last_committed_batch_number
        && batch_number != twine_chain_storage_data.last_finalized_batch_number + 1
    {
        return Err(ProgramCustomError::InvalidBatchFinalizationSequence.into());
    }

    if previous_batch_hash != twine_chain_storage_data.last_finalized_batch_hash {
        return Err(ProgramCustomError::LastFinalizedBatchHashMismatch.into());
    }
   
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
    Ok(twine_chain_storage_data)
}

#[cfg(test)]
use mock_clock::Clock;

#[cfg(test)]
mod mock_clock {
    use solana_program::program_error::ProgramError;

    pub struct Clock {
        pub slot: u64,
    }

    impl Clock {
        pub fn get() -> Result<Clock, ProgramError> {
            Ok(Clock { slot: 1000 })
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::{
//         core::state::BatchInfo,
//         utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES},
//     };
//     use solana_program::{clock::Epoch, rent::Rent, system_program};
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
//     fn test_commit_and_finalize_txn() -> Result<(), Box<dyn std::error::Error>> {
//         let program_id = Pubkey::new_unique();

//         // Get the required accounts
//         let (twine_chain_storage_key, _) = derive_twine_chain_storage(&program_id);
//         let (current_batch_key, _) = derive_commitment_pda(&program_id, 1, 3);
//         let (previous_batch_key, _) = derive_commitment_pda(&program_id, 0, 0);
//         let (role_manager_key, _) = derive_role_manager(&program_id);
//         let twine_operation_handler_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
//         let system_program_id = system_program::id();

//         // Required space for each account
//         let twine_chain_storage_space = TwineChainStorage::LEN;
//         let current_batch_space: usize = 1 + (4 + 3 * BlockInfo::LEN) + 1 + 1;
//         let previous_batch_space: usize = 1 + (4 + 0 * BlockInfo::LEN) + 1 + 1;
//         let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

//         // Setup Account Lamports
//         let rent = Rent::default();
//         let mut twine_chain_storage_lamports = rent.minimum_balance(twine_chain_storage_space);
//         let mut current_batch_lamports = rent.minimum_balance(current_batch_space);
//         let mut previous_batch_lamports = rent.minimum_balance(previous_batch_space);
//         let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
//         let mut twine_operation_handler_lamports = 1_000_000_000;
//         let mut system_program_lamports = 0;

//         // Setup Twine Chain Storage initial data
//         let batch_info = BatchInfo {
//             start_block: 0,
//             end_block: 0,
//         };

//         let committed_info = BatchInfo {
//             start_block: 1,
//             end_block: 3,
//         };

//         let twine_chain_data = TwineChainStorage {
//             is_initialized: true,
//             last_copied_deposit_nonce: 0,
//             last_copied_forced_withdrawal_nonce: 0,
//             groth16_vk: Vec::new(),
//             execution_vkey: String::from(""),
//             inclusion_vkey: String::from(""),
//             withdrawal_vkey: String::from(""),
//             skip_verification: true,
//             last_finalized_batch: batch_info.clone(),
//             last_committed_batch: committed_info,
//             last_transaction_finalized_batch: batch_info.clone(),
//             last_finalized_receipt_root: [0u8; 32],
//         };

//         let mut twine_chain_storage_data = vec![];
//         twine_chain_data.serialize(&mut twine_chain_storage_data)?;

//         // Gives role TwineOperationHandler
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

//         // Setup previous batch account data
//         let previous_block_info = BlockInfo {
//             previous_hash: [0u8; 32],
//             block_hash: [0u8; 32],
//             transaction_root: [0u8; 32],
//             receipt_root: [0u8; 32],
//         };
//         let batch_data_prev = BatchPdaAccount {
//             is_initialized: true,
//             infos: vec![previous_block_info],
//             verified: true,
//             is_full: true,
//         };
//         let mut previous_batch_data = vec![];
//         batch_data_prev.serialize(&mut previous_batch_data)?;

//         // Setup current batch account data
//         let block1_info = BlockInfo {
//             previous_hash: [0u8; 32],
//             block_hash: [1u8; 32],
//             transaction_root: [0u8; 32],
//             receipt_root: [1u8; 32],
//         };
//         let block2_info = BlockInfo {
//             previous_hash: [1u8; 32],
//             block_hash: [2u8; 32],
//             transaction_root: [0u8; 32],
//             receipt_root: [2u8; 32],
//         };
//         let block3_info = BlockInfo {
//             previous_hash: [2u8; 32],
//             block_hash: [3u8; 32],
//             transaction_root: [0u8; 32],
//             receipt_root: [3u8; 32],
//         };
//         let batch_data = vec![block1_info, block2_info, block3_info];

//         let batch_data_curr = BatchPdaAccount {
//             is_initialized: true,
//             infos: batch_data.clone(),
//             verified: false,
//             is_full: true,
//         };
//         let mut current_batch_data = vec![];
//         batch_data_curr.serialize(&mut current_batch_data)?;

//         // Setup remaining account's data
//         let mut twine_operation_handler_data = vec![];
//         let mut system_program_data = vec![];

//         // Setup owners
//         let mut twine_chain_storage_owner = program_id;
//         let mut current_batch_owner = program_id;
//         let mut previous_batch_owner = program_id;
//         let mut role_manager_owner = program_id;
//         let mut twine_operation_handler_owner = system_program_id;
//         let mut system_program_owner = system_program_id;

//         // Create required account infos
//         let twine_chain_storage_account = create_test_account_info(
//             &twine_chain_storage_key,
//             false,
//             true,
//             &mut twine_chain_storage_lamports,
//             &mut twine_chain_storage_data,
//             &mut twine_chain_storage_owner,
//         );

//         let current_batch_account = create_test_account_info(
//             &current_batch_key,
//             false,
//             true,
//             &mut current_batch_lamports,
//             &mut current_batch_data,
//             &mut current_batch_owner,
//         );

//         let previous_batch_account = create_test_account_info(
//             &previous_batch_key,
//             false,
//             true,
//             &mut previous_batch_lamports,
//             &mut previous_batch_data,
//             &mut previous_batch_owner,
//         );

//         let role_manager_account = create_test_account_info(
//             &role_manager_key,
//             false,
//             true,
//             &mut role_manager_lamports,
//             &mut role_manager_data,
//             &mut role_manager_owner,
//         );

//         let twine_operation_handler_account = create_test_account_info(
//             &twine_operation_handler_key,
//             true,
//             false,
//             &mut twine_operation_handler_lamports,
//             &mut twine_operation_handler_data,
//             &mut twine_operation_handler_owner,
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
//             twine_chain_storage_account.clone(),
//             current_batch_account.clone(),
//             previous_batch_account.clone(),
//             role_manager_account.clone(),
//             twine_operation_handler_account.clone(),
//             system_program_account.clone(),
//         ];

//         // Prepare Public Input
//         let start_block: u64 = 1;
//         let end_block: u64 = 3;
//         let mut calculated_batch_hash = [0u8; 32];
//         let mut serialized_batch_hash: Vec<u8> =
//             Vec::with_capacity(BlockInfo::LEN * batch_data.len());

//         for block in batch_data {
//             serialized_batch_hash.extend_from_slice(&block.abi_encode_packed());
//         }
//         let mut hasher = Keccak256::new();
//         hasher.update(serialized_batch_hash);

//         let encoded_batch_hash = hasher.finalize().to_vec();
//         calculated_batch_hash[..32].copy_from_slice(&encoded_batch_hash[..32]);

//         let mut public_values = Vec::with_capacity(48);
//         public_values.extend_from_slice(&start_block.to_be_bytes());
//         public_values.extend_from_slice(&end_block.to_be_bytes());
//         public_values.extend_from_slice(&calculated_batch_hash);

//         // Call finalize Batch
//         let result = finalize_batch(&program_id, &accounts, public_values.clone(), public_values);
//         assert!(result.is_ok(), "Finalization Failed: {:?}", result.err());

//         // verify commitments
//         let current_batch_data =
//             BatchPdaAccount::deserialize(&mut &current_batch_account.data.borrow()[..])?;

//         assert!(current_batch_data.verified, "Batch should be verified");

//         let twine_chain_storage_data =
//             TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data.borrow()[..])?;

//         assert_eq!(
//             twine_chain_storage_data.last_finalized_batch.start_block, 1,
//             "Last batch's start block should be 1"
//         );

//         assert_eq!(
//             twine_chain_storage_data.last_finalized_batch.end_block, 3,
//             "Last batch's end block should be 3"
//         );

//         Ok(())
//     }
// }
