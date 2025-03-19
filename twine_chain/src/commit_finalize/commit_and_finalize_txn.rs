use crate::core::error::ProgramCustomError;
use crate::core::state::{
    BatchPdaAccount, BlockInfo, ChainCommitment, DepositMessageInfo, DepositMessagesBuffer,
    ExecutionMessageBuffer, ForcedWithdrawMessageInfo, ForcedWithdrawMessagesBuffer,
    LayerZeroMessageInfo, LayerZeroMessagesBuffer, RoleType, TwineChainRoleManager,
    TwineChainStorage,
};
use crate::utils::constants::{
    COMMITMENT_PDA_PREFIX, DEPOSIT_BUFFER_PREFIX, EXECUTION_BUFFER_PREFIX,
    FORCED_WITHDRAWAL_BUFFER_PREFIX, LAYER_ZERO_BUFFER_PREFIX, ROLE_MANAGER_PREFIX,
    TWINE_CHAIN_STORAGE_PREFIX,
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

pub fn commit_and_finalize_transaction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    transaction_info: Vec<u8>,
    inclusion_proof: Vec<u8>,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let current_batch = next_account_info(account_iter)?;
    let twine_chain_storage = next_account_info(account_iter)?;
    let deposit_message_buffer = next_account_info(account_iter)?;
    let forced_withdrawal_message_buffer = next_account_info(account_iter)?;
    let layer_zero_message_buffer = next_account_info(account_iter)?;
    let execution_message_buffer = next_account_info(account_iter)?;
    let role_manager = next_account_info(account_iter)?;
    let twine_operation_handler = next_account_info(account_iter)?;

    // Validate signer
    if !twine_operation_handler.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Extract and decode the first 48 bytes
    let (first_48_bytes, _rest) = transaction_info.split_at(48);
    let (start_block, end_block, combined_receipt_root) = decode_batch_info(first_48_bytes);

    // Validate PDAs
    validate_pdas(
        program_id,
        start_block,
        end_block,
        current_batch,
        twine_chain_storage,
        deposit_message_buffer,
        forced_withdrawal_message_buffer,
        layer_zero_message_buffer,
        execution_message_buffer,
        role_manager,
    )?;

    // Check if initiator has TwineOperationHandler Role
    let role_manager_data = TwineChainRoleManager::try_from_slice(&role_manager.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(twine_operation_handler.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    // Check if current batch is initialized
    let current_batch_data = BatchPdaAccount::try_from_slice(&current_batch.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !current_batch_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    // Check if batch is verified
    if !current_batch_data.verified {
        return Err(ProgramCustomError::BatchNotFinalized.into());
    }

    // Calculate and check combined receipt root
    let calculated_combined_receipt_root =
        calculate_combined_receipt_root(&current_batch_data.infos);

    if calculated_combined_receipt_root != combined_receipt_root {
        return Err(ProgramCustomError::InvalidReceiptRoot.into());
    }

    // Deserialize message buffers
    let mut deposit_buffer_data =
        DepositMessagesBuffer::try_from_slice(&deposit_message_buffer.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut withdraw_buffer_data = ForcedWithdrawMessagesBuffer::try_from_slice(
        &forced_withdrawal_message_buffer.data.borrow(),
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut lz_buffer_data =
        LayerZeroMessagesBuffer::try_from_slice(&layer_zero_message_buffer.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut execution_buffer_data =
        ExecutionMessageBuffer::try_from_slice(&execution_message_buffer.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Extract and decode the transaction information of solana
    let chain_data_bytes = &transaction_info[168..288];
    let decoded_chain_data = decode_chain_commitment(chain_data_bytes);

    let deposit_count = decoded_chain_data.deposit_count as usize;
    let withdraw_count = decoded_chain_data.withdraw_count as usize;
    let lz_transaction_count = decoded_chain_data.lz_transaction_count as usize;

    if deposit_count > deposit_buffer_data.deposit_messages.len()
        || withdraw_count > withdraw_buffer_data.withdraw_messages.len()
        || lz_transaction_count > lz_buffer_data.lz_messages.len()
    {
        return Err(ProgramCustomError::GreaterCount.into());
    }

    // Calculate and Check Rolling Hashes
    let mut deposit_rolling_hash: [u8; 32] = [0u8; 32];
    if deposit_count != 0 {
        let selected_deposits = &deposit_buffer_data.deposit_messages[0..deposit_count];
        deposit_rolling_hash = calculate_deposit_rolling_hash(selected_deposits);
    }
    if deposit_rolling_hash != decoded_chain_data.deposit_rolling_hash {
        return Err(ProgramCustomError::DepositRollingHashMismatch.into());
    }

    let mut withdraw_rolling_hash: [u8; 32] = [0u8; 32];
    if withdraw_count != 0 {
        let selected_withdrawals = &withdraw_buffer_data.withdraw_messages[0..withdraw_count];
        withdraw_rolling_hash = calculate_withdraw_rolling_hash(selected_withdrawals);
    }
    if withdraw_rolling_hash != decoded_chain_data.withdraw_rolling_hash {
        return Err(ProgramCustomError::WithdrawRollingHashMismatch.into());
    }

    let mut lz_transaction_rolling_hash: [u8; 32] = [0u8; 32];
    if lz_transaction_count != 0 {
        let selected_lz_transactions = &lz_buffer_data.lz_messages[0..lz_transaction_count];
        lz_transaction_rolling_hash = calculate_layerzero_rolling_hash(selected_lz_transactions);
    }
    if lz_transaction_rolling_hash != decoded_chain_data.lz_transaction_rolling_hash {
        return Err(ProgramCustomError::LayerZeroRollingHashMismatch.into());
    }

    // Deserialize Twine chain storage's data
    let mut twine_chain_storage_data =
        TwineChainStorage::try_from_slice(&twine_chain_storage.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify Inclusion Proof
    verify_proof(
        &inclusion_proof,
        &transaction_info,
        &twine_chain_storage_data.execution_vkey,
        GROTH16_VK_4_0_0_RC3_BYTES,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Move the withdrawals that are ready for execution to execution queue
    for i in 0..withdraw_count {
        let forced_message = withdraw_buffer_data
            .withdraw_messages
            .get(i)
            .ok_or(ProgramCustomError::InvalidIndex)?;
        execution_buffer_data
            .withdrawals
            .push(forced_message.clone());
    }

    // Remove Deposit, Withdrawals and LayerZero messages from queue
    deposit_buffer_data.deposit_messages.drain(0..deposit_count);
    withdraw_buffer_data
        .withdraw_messages
        .drain(0..withdraw_count);
    lz_buffer_data.lz_messages.drain(0..lz_transaction_count);

    twine_chain_storage_data
        .last_transcation_finalized_batch
        .start_block = start_block;
    twine_chain_storage_data
        .last_transcation_finalized_batch
        .end_block = end_block;

    // Serialize data and update the PDAs
    deposit_buffer_data
        .serialize(&mut &mut deposit_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    withdraw_buffer_data
        .serialize(&mut &mut forced_withdrawal_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    lz_buffer_data
        .serialize(&mut &mut layer_zero_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    execution_buffer_data
        .serialize(&mut &mut execution_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    Ok(())
}

fn decode_batch_info(first_48_bytes: &[u8]) -> (u64, u64, [u8; 32]) {
    let start_block: u64 = u64::from_be_bytes(first_48_bytes[0..8].try_into().unwrap());
    let end_block: u64 = u64::from_be_bytes(first_48_bytes[8..16].try_into().unwrap());
    let combined_receipt_root: [u8; 32] = first_48_bytes[16..48].try_into().unwrap();

    return (start_block, end_block, combined_receipt_root);
}

fn calculate_combined_receipt_root(batch_info: &Vec<BlockInfo>) -> [u8; 32] {
    let mut calculated_receipt_root = [0u8; 32];

    let mut receipt_root_vector = Vec::new();
    for blocks in batch_info {
        receipt_root_vector.extend_from_slice(&blocks.receipt_root);
    }

    let mut hasher = Keccak256::new();
    hasher.update(receipt_root_vector);
    let receipt_root = hasher.finalize().to_vec();

    calculated_receipt_root[..32].copy_from_slice(&receipt_root[..32]);

    calculated_receipt_root
}

fn decode_chain_commitment(chain_data_bytes: &[u8]) -> ChainCommitment {
    let deposit_count = u64::from_be_bytes(chain_data_bytes[0..8].try_into().unwrap());
    let deposit_rolling_hash: [u8; 32] = chain_data_bytes[8..40].try_into().unwrap();
    let withdraw_count = u64::from_be_bytes(chain_data_bytes[40..48].try_into().unwrap());
    let withdraw_rolling_hash: [u8; 32] = chain_data_bytes[48..80].try_into().unwrap();
    let lz_transaction_count = u64::from_be_bytes(chain_data_bytes[80..88].try_into().unwrap());
    let lz_transaction_rolling_hash: [u8; 32] = chain_data_bytes[88..120].try_into().unwrap();

    let chain_info = ChainCommitment {
        deposit_count: deposit_count,
        deposit_rolling_hash: deposit_rolling_hash,
        withdraw_count: withdraw_count,
        withdraw_rolling_hash: withdraw_rolling_hash,
        lz_transaction_count: lz_transaction_count,
        lz_transaction_rolling_hash: lz_transaction_rolling_hash,
    };

    return chain_info;
}

fn calculate_deposit_rolling_hash(selected_deposits: &[DepositMessageInfo]) -> [u8; 32] {
    let mut deposit_rolling_hash = [0u8; 32];
    let mut serialized_deposit_data = Vec::new();

    for deposits in selected_deposits {
        serialized_deposit_data.extend_from_slice(&deposits.abi_encode_packed());
    }

    let mut hasher = Keccak256::new();
    hasher.update(serialized_deposit_data);
    let combined_deposit_hash = hasher.finalize().to_vec();

    deposit_rolling_hash[..32].copy_from_slice(&combined_deposit_hash[..32]);

    return deposit_rolling_hash;
}

fn calculate_withdraw_rolling_hash(selected_withdrawals: &[ForcedWithdrawMessageInfo]) -> [u8; 32] {
    let mut withdraw_rolling_hash = [0u8; 32];
    let mut serialized_withdraw_data = Vec::new();

    for withdrawals in selected_withdrawals {
        serialized_withdraw_data.extend_from_slice(&withdrawals.abi_encode_packed());
    }

    let mut hasher = Keccak256::new();
    hasher.update(serialized_withdraw_data);
    let combined_withdraw_hash = hasher.finalize().to_vec();

    withdraw_rolling_hash[..32].copy_from_slice(&combined_withdraw_hash[..32]);

    return withdraw_rolling_hash;
}

fn calculate_layerzero_rolling_hash(selected_lz_messages: &[LayerZeroMessageInfo]) -> [u8; 32] {
    let mut lz_rolling_hash = [0u8; 32];
    let mut serialized_lz_data = Vec::new();

    for messages in selected_lz_messages {
        serialized_lz_data.extend_from_slice(&messages.abi_encode_packed());
    }

    let mut hasher = Keccak256::new();
    hasher.update(serialized_lz_data);
    let combined_withdraw_hash = hasher.finalize().to_vec();

    lz_rolling_hash[..32].copy_from_slice(&combined_withdraw_hash[..32]);

    return lz_rolling_hash;
}

fn validate_pdas(
    program_id: &Pubkey,
    start_block: u64,
    end_block: u64,
    current_batch: &AccountInfo,
    twine_chain_storage: &AccountInfo,
    deposit_message_buffer: &AccountInfo,
    forced_withdrawal_message_buffer: &AccountInfo,
    layer_zero_message_buffer: &AccountInfo,
    execution_message_buffer: &AccountInfo,
    role_manager: &AccountInfo,
) -> ProgramResult {
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

    // Derive and validate depositMessageBuffer PDA
    let (expected_deposit_message_pda, _deposit_pda_bump) =
        Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_deposit_message_pda != *deposit_message_buffer.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Derive and validate forcedWithdrawMessageBuffer PDA
    let (expected_withdraw_message_pda, _withdraw_pda_bump) =
        Pubkey::find_program_address(&[FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_withdraw_message_pda != *forced_withdrawal_message_buffer.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Derive and validate layerZeroMessageBuffer PDA
    let (expected_lz_message_pda, _lz_pda_bump) =
        Pubkey::find_program_address(&[LAYER_ZERO_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_lz_message_pda != *layer_zero_message_buffer.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Derive and validate ExecutionMessageBuffer PDA
    let (expected_execution_message_pda, _execution_pda_bump) =
        Pubkey::find_program_address(&[EXECUTION_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_execution_message_pda != *execution_message_buffer.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    Ok(())
}
