use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::clock::Clock;
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
    sysvar::Sysvar,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            BatchPdaAccount, BlockInfo, CommitBatchInfo, RoleType, TwineChainRoleManager,
            TwineChainStorage,
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
    start_block: u64,
    end_block: u64,
    batch_data: Vec<CommitBatchInfo>,
) -> ProgramResult {
    if batch_data.len() == 0 {
        return Err(ProgramCustomError::EmptyBatchCommitment.into());
    }

    let account_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let current_batch_acc = next_account_info(account_iter)?;
    let previous_batch_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let twine_operation_handler_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Validate PDAs
    let (current_pda_bump, mut twine_chain_storage_data) = validate_pdas(
        program_id,
        start_block,
        end_block,
        twine_chain_storage_acc,
        current_batch_acc,
        previous_batch_acc,
        role_manager_acc,
        twine_operation_handler_acc,
        system_program,
    )?;

    if start_block != twine_chain_storage_data.last_committed_batch.end_block + 1 {
        return Err(ProgramCustomError::InvalidBlockCommitmentSequence.into());
    }

    // Initialize commitment PDA if not already initialized
    if current_batch_acc.data_is_empty() {
        let rent = Rent::default();
        let batch_space =
            1 + (4 + ((end_block as usize - start_block as usize) + 1) * BlockInfo::LEN) + 1 + 1;
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
                &start_block.to_be_bytes(),
                &end_block.to_be_bytes(),
                &[current_pda_bump],
            ]],
        )?;
        let batch_data = BatchPdaAccount {
            is_initialized: true,
            infos: Vec::new(),
            verified: false,
            is_full: true,
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

    if current_batch_data.infos.len() == 0 {
        if batch_data[0].block_number != start_block {
            return Err(ProgramCustomError::InvalidBlockData.into());
        }
    }

    let previous_batch_data =
        BatchPdaAccount::deserialize(&mut &previous_batch_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut last_block = 0;
    for blocks in batch_data {
        let previous_block_hash = match current_batch_data.infos.last() {
            Some(last_info) => last_info.block_hash,
            None => match previous_batch_data.infos.last() {
                Some(last_info) => last_info.block_hash,
                None => return Err(ProgramCustomError::EmptyPreviousBatch.into()),
            },
        };
        current_batch_data.infos.push(BlockInfo {
            previous_hash: previous_block_hash,
            block_hash: blocks.block_hash,
            transaction_root: blocks.transaction_root,
            receipt_root: blocks.receipt_root,
        });
        last_block = blocks.block_number;
    }

    if last_block > end_block {
        return Err(ProgramCustomError::InvalidBlockData.into());
    }

    // Update current batch
    current_batch_data
        .serialize(&mut &mut current_batch_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    if last_block == end_block {
        twine_chain_storage_data.last_committed_batch.start_block = start_block;
        twine_chain_storage_data.last_committed_batch.end_block = end_block;

        current_batch_data.is_full = true;

        // Update twine chain storage
        twine_chain_storage_data
            .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
            .map_err(|_| ProgramCustomError::SerializeFailed)?;

        // Emit event
        let clock = Clock::get()?;
        msg!(
            "event=BatchCommitmentSuccessful start_block={} end_block={} chain_id={} slot_number={}",
            start_block,
            end_block,
            CHAIN_ID,
            clock.slot
        );
    }

    Ok(())
}

fn validate_pdas(
    program_id: &Pubkey,
    start_block: u64,
    end_block: u64,
    twine_chain_storage_acc: &AccountInfo,
    current_batch_acc: &AccountInfo,
    previous_batch_acc: &AccountInfo,
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

    let (expected_current_pda, current_pda_bump) =
        derive_commitment_pda(program_id, start_block, end_block);
    verify_derived_address(expected_current_pda, current_batch_acc)?;

    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Derive and validate previous batch PDA
    let (expected_previous_pda, _) = derive_commitment_pda(
        program_id,
        twine_chain_storage_data.last_committed_batch.start_block,
        twine_chain_storage_data.last_committed_batch.end_block,
    );
    verify_derived_address(expected_previous_pda, previous_batch_acc)?;

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
        let space = 1 + (4 + 3 * BlockInfo::LEN) + 1 + 1;
        let leaked: &'static mut [u8] = Box::leak(vec![0u8; space].into_boxed_slice());
        unsafe {
            let mut data_ref = acc.data.borrow_mut();
            *data_ref = mem::transmute::<&'static mut [u8], &mut [u8]>(leaked);
        }
    }
    Ok(())
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        core::state::BatchInfo,
        utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES},
    };
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
    fn test_commit_batch() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (twine_chain_storage_key, _) = derive_twine_chain_storage(&program_id);
        let (current_batch_key, _) = derive_commitment_pda(&program_id, 1, 3);
        let (previous_batch_key, _) = derive_commitment_pda(&program_id, 0, 0);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let twine_operation_handler_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let twine_chain_storage_space = TwineChainStorage::LEN;
        let current_batch_space: usize = 1 + (4 + 3 * BlockInfo::LEN) + 1 + 1;
        let previous_batch_space: usize = 1 + (4 + 0 * BlockInfo::LEN) + 1 + 1;
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut twine_chain_storage_lamports = rent.minimum_balance(twine_chain_storage_space);
        let mut current_batch_lamports = rent.minimum_balance(current_batch_space);
        let mut previous_batch_lamports = rent.minimum_balance(previous_batch_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut twine_operation_handler_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup Twine Chain Storage initial data
        let batch_info = BatchInfo {
            start_block: 0,
            end_block: 0,
        };

        let twine_chain_data = TwineChainStorage {
            is_initialized: true,
            last_copied_deposit_nonce: 0,
            last_copied_forced_withdrawal_nonce: 0,
            groth16_vk: Vec::new(),
            execution_vkey: String::from(""),
            inclusion_vkey: String::from(""),
            withdrawal_vkey: String::from(""),
            skip_verification: true,
            last_finalized_batch: batch_info.clone(),
            last_committed_batch: batch_info.clone(),
            last_transaction_finalized_batch: batch_info.clone(),
            last_finalized_receipt_root: [0u8; 32],
        };

        let mut twine_chain_storage_data = vec![];
        twine_chain_data.serialize(&mut twine_chain_storage_data)?;

        // Gives role TwineOperationHandler
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

        // Setup previous batch account data
        let previous_block_info = BlockInfo {
            previous_hash: [0u8; 32],
            block_hash: [1u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [0u8; 32],
        };
        let batch_data = BatchPdaAccount {
            is_initialized: true,
            infos: vec![previous_block_info],
            verified: true,
            is_full: true,
        };
        let mut previous_batch_data = vec![];
        batch_data.serialize(&mut previous_batch_data)?;

        // Setup remaining account's data
        let mut current_batch_data = vec![];
        let mut twine_operation_handler_data = vec![];
        let mut system_program_data = vec![];

        // Setup owners
        let mut twine_chain_storage_owner = program_id;
        let mut current_batch_owner = program_id;
        let mut previous_batch_owner = program_id;
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

        let current_batch_account: AccountInfo<'_> = create_test_account_info(
            &current_batch_key,
            false,
            true,
            &mut current_batch_lamports,
            &mut current_batch_data,
            &mut current_batch_owner,
        );

        let previous_batch_account = create_test_account_info(
            &previous_batch_key,
            false,
            true,
            &mut previous_batch_lamports,
            &mut previous_batch_data,
            &mut previous_batch_owner,
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
            current_batch_account.clone(),
            previous_batch_account.clone(),
            role_manager_account.clone(),
            twine_operation_handler_account.clone(),
            system_program_account.clone(),
        ];

        // Call the initialize function
        let block1_info = CommitBatchInfo {
            block_number: 1,
            block_hash: [2u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [1u8; 32],
        };
        let block2_info = CommitBatchInfo {
            block_number: 2,
            block_hash: [3u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [2u8; 32],
        };
        let block3_info = CommitBatchInfo {
            block_number: 3,
            block_hash: [4u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [3u8; 32],
        };

        let batch_data = vec![block1_info, block2_info, block3_info];
        let result = commit_batch(&program_id, &accounts, 1, 3, batch_data);
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        // verify commitments
        let current_batch_data =
            BatchPdaAccount::deserialize(&mut &current_batch_account.data.borrow()[..])?;

        assert!(
            current_batch_data.is_initialized,
            "Batch Account should be initialized"
        );

        assert!(current_batch_data.is_full, "is_full should be set to true");

        assert_eq!(
            current_batch_data.infos.len(),
            3,
            "There should data for 3 blocks"
        );

        assert!(!current_batch_data.verified, "Batch should not be verified");

        let twine_chain_storage_data =
            TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data.borrow()[..])?;

        assert_eq!(
            twine_chain_storage_data.last_committed_batch.start_block, 1,
            "Last batch's start block should be 1"
        );

        assert_eq!(
            twine_chain_storage_data.last_committed_batch.end_block, 3,
            "Last batch's end block should be 3"
        );

        Ok(())
    }
}
