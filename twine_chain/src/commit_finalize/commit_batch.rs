use crate::core::error::ProgramCustomError;
use crate::core::state::{
    BatchPdaAccount, BlockInfo, CommitBatchInfo, RoleType, TwineChainRoleManager, TwineChainStorage,
};
use crate::utils::constants::{
    COMMITMENT_PDA_PREFIX, DISCRIMINATOR, MAX_QUEUE_SIZE, ROLE_MANAGER_PREFIX,
    TWINE_CHAIN_STORAGE_PREFIX,
};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
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
    let twine_chain_storage = next_account_info(account_iter)?;
    let current_batch = next_account_info(account_iter)?;
    let previous_batch = next_account_info(account_iter)?;
    let role_manager = next_account_info(account_iter)?;
    let twine_operation_handler = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Validate signer
    if !twine_operation_handler.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate PDAs
    let (current_pda_bump, mut twine_chain_storage_data) = validate_pdas(
        program_id,
        start_block,
        end_block,
        twine_chain_storage,
        current_batch,
        previous_batch,
        role_manager,
    )?;

    // Check if initiator has TwineOperationHandler Role
    let role_manager_data = TwineChainRoleManager::try_from_slice(&role_manager.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(twine_operation_handler.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    if start_block != twine_chain_storage_data.last_committed_batch.end_block + 1 {
        return Err(ProgramCustomError::InvalidBlockCommitmentSequence.into());
    }

    // Initialize commitment PDA if not already initialized
    if current_batch.lamports() == 0 {
        let rent = Rent::get()?;
        let batch_space = DISCRIMINATOR + 1 + (4 + MAX_QUEUE_SIZE * BlockInfo::LEN) + 1 + 1;
        let required_lamports = rent.minimum_balance(batch_space);
        let create_ix = system_instruction::create_account(
            twine_operation_handler.key,
            current_batch.key,
            required_lamports,
            batch_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                twine_operation_handler.clone(),
                current_batch.clone(),
                system_program.clone(),
            ],
            &[&[COMMITMENT_PDA_PREFIX.as_bytes(), &[current_pda_bump]]],
        )?;

        let mut newly_initialized_batch_data =
            BatchPdaAccount::try_from_slice(&current_batch.data.borrow())
                .map_err(|_| ProgramError::InvalidAccountData)?;

        if newly_initialized_batch_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        newly_initialized_batch_data.is_initialized = true;
        newly_initialized_batch_data
            .serialize(&mut &mut current_batch.data.borrow_mut()[..])
            .map_err(|_| ProgramCustomError::SerializeFailed)?;
    }

    let mut current_batch_data = BatchPdaAccount::try_from_slice(&current_batch.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !current_batch_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    if current_batch_data.infos.len() == 0 {
        if batch_data[0].block_number != start_block {
            return Err(ProgramCustomError::InvalidBlockData.into());
        }
    }

    let previous_batch_data = BatchPdaAccount::try_from_slice(&previous_batch.data.borrow())
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

    if last_block == end_block {
        twine_chain_storage_data.last_committed_batch.start_block = start_block;
        twine_chain_storage_data.last_committed_batch.end_block = end_block;

        current_batch_data.is_full = true;
        // TODO: Emit BatchCommitmentSuccessful event
    }

    // Update current batch and twine chain storage
    current_batch_data
        .serialize(&mut &mut current_batch.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}

fn validate_pdas(
    program_id: &Pubkey,
    start_block: u64,
    end_block: u64,
    twine_chain_storage: &AccountInfo,
    current_batch: &AccountInfo,
    previous_batch: &AccountInfo,
    role_manager: &AccountInfo,
) -> Result<(u8, TwineChainStorage), ProgramError> {
    // Dervie and validate Twine Chain Storage PDA
    let (storage_pda, _storage_bump) =
        Pubkey::find_program_address(&[TWINE_CHAIN_STORAGE_PREFIX.as_bytes()], program_id);

    if storage_pda != *twine_chain_storage.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Derive and validate Role Manager PDA
    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Derive and validate current batch PDA
    let (current_pda, current_pda_bump) = Pubkey::find_program_address(
        &[
            COMMITMENT_PDA_PREFIX.as_bytes(),
            &start_block.to_be_bytes(),
            &end_block.to_be_bytes(),
        ],
        program_id,
    );

    if current_pda != *current_batch.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::try_from_slice(&twine_chain_storage.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Derive and validate previous batch PDA
    let (previous_pda, _previous_pda_seed) = Pubkey::find_program_address(
        &[
            COMMITMENT_PDA_PREFIX.as_bytes(),
            &twine_chain_storage_data
                .last_committed_batch
                .start_block
                .to_be_bytes(),
            &twine_chain_storage_data
                .last_committed_batch
                .end_block
                .to_be_bytes(),
        ],
        program_id,
    );

    if previous_pda != *previous_batch.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    Ok((current_pda_bump, twine_chain_storage_data))
}
