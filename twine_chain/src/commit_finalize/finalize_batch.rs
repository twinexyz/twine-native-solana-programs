use borsh::{BorshDeserialize, BorshSerialize};
use serde_json;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
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
        state::{
            BatchPdaAccount, FinalizedBatchEvent, RoleType, TwineChainRoleManager,
            TwineChainStorage,
        },
    },
    utils::{
        address_derivation::{
            derive_commitment_pda, derive_twine_chain_role_manager, derive_twine_chain_storage,
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

    let mut twine_chain_storage_data = validate_accounts(
        program_id,
        batch_number,
        previous_batch_hash,
        current_batch_hash,
        twine_chain_storage_acc,
        current_batch_acc,
        role_manager_acc,
        twine_operation_handler_acc,
    )?;

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
    twine_chain_storage_data.total_msg_handled_on_twine = executed_message_count;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let clock = Clock::get()?;

    let event = FinalizedBatchEvent {
        event: "FinalizedBatch".to_string(),
        batch_number: batch_number,
        messages_handled_on_twine: executed_message_count,
        chain_id: CHAIN_ID,
        slot_number: clock.slot,
        batch_hash: current_batch_hash,   
    };
    let serialized_event =
        serde_json::to_string(&event).map_err(|_| ProgramCustomError::FailedToSerializeEvent)?;
    msg!("{}", serialized_event);

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

fn validate_accounts(
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

    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    let (expected_current_pda, _current_pda_bump) = derive_commitment_pda(program_id, batch_number);
    verify_derived_address(expected_current_pda, current_batch_acc)?;
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
    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if batch_number != twine_chain_storage_data.last_finalized_batch_number + 1 {
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
