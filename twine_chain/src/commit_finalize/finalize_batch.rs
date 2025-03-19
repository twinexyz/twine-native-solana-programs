use crate::core::error::ProgramCustomError;
use crate::core::state::{
    BatchPdaAccount, BlockInfo, RoleType, TwineChainRoleManager, TwineChainStorage,
};
use crate::utils::constants::{
    COMMITMENT_PDA_PREFIX, ROLE_MANAGER_PREFIX, TWINE_CHAIN_STORAGE_PREFIX,
};
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
use solana_program::program_pack::IsInitialized;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};

pub fn finalize_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let twine_chain_storage = next_account_info(account_iter)?;
    let current_batch = next_account_info(account_iter)?;
    let previous_batch = next_account_info(account_iter)?;
    let role_manager = next_account_info(account_iter)?;
    let twine_operation_handler = next_account_info(account_iter)?;

    let (start_block, end_block, decoded_batch_hash) = decode_batch_info(&public_values);

    // Validate signer
    if !twine_operation_handler.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut twine_chain_storage_data = validate_pdas(
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

    // Checking if the block sequence is correct
    if start_block != twine_chain_storage_data.last_finalized_batch.end_block + 1 {
        return Err(ProgramCustomError::InvalidBlockFinalizationSequence.into());
    }

    // Checking if previous batch is finalized
    let previous_batch_data = BatchPdaAccount::try_from_slice(&previous_batch.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !previous_batch_data.verified {
        return Err(ProgramCustomError::PreviousBatchNotFinalized.into());
    }

    // Checking if batch is filled
    let mut current_batch_data = BatchPdaAccount::try_from_slice(&current_batch.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !current_batch_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }
    if !current_batch_data.is_full {
        return Err(ProgramCustomError::BatchNotFilled.into());
    }

    // Checking if provided batch hash is correct
    let calculated_batch_hash = compute_batch_hash(&current_batch_data.infos);
    if calculated_batch_hash != decoded_batch_hash {
        return Err(ProgramCustomError::BatchHashMismatch.into());
    }

    // Calling SP1 Verifier to verify the execution proof
    // TODO: Provide the groth16_vk from twine chain storage instead.
    // so that the sp1 verifier version can be changed without upgrading prorgams.
    verify_proof(
        &execution_proof,
        &public_values,
        &twine_chain_storage_data.execution_vkey,
        GROTH16_VK_4_0_0_RC3_BYTES,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Updating the states
    current_batch_data.verified = true;
    twine_chain_storage_data.last_finalized_batch.start_block = start_block;
    twine_chain_storage_data.last_finalized_batch.end_block = end_block;

    current_batch_data
        .serialize(&mut &mut twine_chain_storage.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}

fn decode_batch_info(batch_info: &Vec<u8>) -> (u64, u64, [u8; 32]) {
    let start_block = u64::from_be_bytes(batch_info[0..8].try_into().unwrap());
    let end_block = u64::from_be_bytes(batch_info[8..16].try_into().unwrap());
    let batch_hash: [u8; 32] = batch_info[16..48].try_into().unwrap();

    return (start_block, end_block, batch_hash);
}

fn compute_batch_hash(batch_info: &Vec<BlockInfo>) -> [u8; 32] {
    let mut calculated_batch_hash = [0u8; 32];
    let mut serialized_batch_hash: Vec<u8> = Vec::with_capacity(BlockInfo::LEN * batch_info.len());

    for block in batch_info {
        serialized_batch_hash.extend_from_slice(&block.abi_encode_packed());
    }

    let mut hasher = Keccak256::new();
    hasher.update(serialized_batch_hash);
    let encoded_batch_hash = hasher.finalize().to_vec();

    calculated_batch_hash[..32].copy_from_slice(&encoded_batch_hash[..32]);

    calculated_batch_hash
}

fn validate_pdas(
    program_id: &Pubkey,
    start_block: u64,
    end_block: u64,
    twine_chain_storage: &AccountInfo,
    current_batch: &AccountInfo,
    previous_batch: &AccountInfo,
    role_manager: &AccountInfo,
) -> Result<TwineChainStorage, ProgramError> {
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
    let (current_pda, _current_pda_bump) = Pubkey::find_program_address(
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

    Ok(twine_chain_storage_data)
}
