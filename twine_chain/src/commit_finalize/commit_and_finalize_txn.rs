use crate::core::error::ProgramCustomError;
use crate::core::state::{
    BatchPdaAccount, BlockInfo, ChainCommitment, DepositMessageInfo, DepositMessagesBuffer,
    ExecutionMessageBuffer, ForcedWithdrawMessageInfo, ForcedWithdrawMessagesBuffer,
    LayerZeroMessageInfo, LayerZeroMessagesBuffer, RoleType, TwineChainRoleManager,
    TwineChainStorage,
};
use crate::utils::address_derivation::{
    derive_commitment_pda, derive_deposit_message_buffer, derive_execution_message_buffer,
    derive_forced_withdraw_message_buffer, derive_layer_zero_message_buffer, derive_role_manager,
    derive_twine_chain_storage, verify_derived_address,
};
use crate::utils::constants::CHAIN_ID;
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
#[cfg(not(test))]
use solana_program::clock::Clock;
use solana_program::program_pack::IsInitialized;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    sysvar::Sysvar,
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
    let twine_chain_storage = next_account_info(account_iter)?;
    let current_batch = next_account_info(account_iter)?;
    let deposit_message_buffer = next_account_info(account_iter)?;
    let forced_withdrawal_message_buffer = next_account_info(account_iter)?;
    let layer_zero_message_buffer = next_account_info(account_iter)?;
    let execution_message_buffer = next_account_info(account_iter)?;
    let role_manager = next_account_info(account_iter)?;
    let twine_operation_handler = next_account_info(account_iter)?;

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
        twine_operation_handler,
    )?;

    // Check if current batch is initialized
    let current_batch_data = BatchPdaAccount::deserialize(&mut &current_batch.data.borrow()[..])
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
        DepositMessagesBuffer::deserialize(&mut &deposit_message_buffer.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut withdraw_buffer_data = ForcedWithdrawMessagesBuffer::deserialize(
        &mut &forced_withdrawal_message_buffer.data.borrow()[..],
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut lz_buffer_data =
        LayerZeroMessagesBuffer::deserialize(&mut &layer_zero_message_buffer.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut execution_buffer_data =
        ExecutionMessageBuffer::deserialize(&mut &execution_message_buffer.data.borrow()[..])
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
        TwineChainStorage::deserialize(&mut &twine_chain_storage.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify Inclusion Proof
    if !twine_chain_storage_data.skip_verification {
        verify_proof(
            &inclusion_proof,
            &transaction_info,
            &twine_chain_storage_data.execution_vkey,
            GROTH16_VK_4_0_0_RC3_BYTES,
        )
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    }

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
        .last_transaction_finalized_batch
        .start_block = start_block;
    twine_chain_storage_data
        .last_transaction_finalized_batch
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

    // Emit event
    let clock = Clock::get()?;
    msg!(
        "event=TransactionFinalizationSuccessful start_block={} end_block={} chain_id={} deposit_count={:?} withdraw_count={} slot_number={}",
        start_block,
        end_block,
        CHAIN_ID,
        decoded_chain_data.deposit_count,
        decoded_chain_data.withdraw_count,
        clock.slot
    );
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
    current_batch_acc: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    deposit_messages_buffer_acc: &AccountInfo,
    forced_withdrawal_messages_buffer_acc: &AccountInfo,
    layer_zero_messages_buffer_acc: &AccountInfo,
    execution_messages_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    twine_operation_handler: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !twine_operation_handler.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // Dervie and validate PDAs
    let (expected_current_pda, _) = derive_commitment_pda(program_id, start_block, end_block);
    verify_derived_address(expected_current_pda, current_batch_acc)?;

    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Dervie and validate message PDAs
    let (expected_deposit_pda, _) = derive_deposit_message_buffer(program_id);
    verify_derived_address(expected_deposit_pda, deposit_messages_buffer_acc)?;

    let (expected_forced_withdrawal_pda, _) = derive_forced_withdraw_message_buffer(program_id);
    verify_derived_address(
        expected_forced_withdrawal_pda,
        forced_withdrawal_messages_buffer_acc,
    )?;

    let (expected_layer_zero_pda, _) = derive_layer_zero_message_buffer(program_id);
    verify_derived_address(expected_layer_zero_pda, layer_zero_messages_buffer_acc)?;

    let (expected_execution_pda, _) = derive_execution_message_buffer(program_id);
    verify_derived_address(expected_execution_pda, execution_messages_buffer_acc)?;

    // Check if initiator has TwineOperationHandler Role
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(twine_operation_handler.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
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
        utils::constants::{INITIAL_CHAIN_ADMIN, MAX_QUEUE_SIZE, MAX_ROLES},
    };
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
    fn test_finalize_batch() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (twine_chain_storage_key, _) = derive_twine_chain_storage(&program_id);
        let (current_batch_key, _) = derive_commitment_pda(&program_id, 1, 3);
        let (deposit_buffer_key, _) = derive_deposit_message_buffer(&program_id);
        let (withdraw_buffer_key, _) = derive_forced_withdraw_message_buffer(&program_id);
        let (layer_zero_buffer_key, _) = derive_layer_zero_message_buffer(&program_id);
        let (execution_buffer_key, _) = derive_execution_message_buffer(&program_id);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let twine_operation_handler_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let twine_chain_storage_space = TwineChainStorage::LEN;
        let current_batch_space: usize = 1 + (4 + 3 * BlockInfo::LEN) + 1 + 1;
        let deposit_buffer_space = 1 + 8 + 4 + (MAX_QUEUE_SIZE * DepositMessageInfo::LEN);
        let withdraw_buffer_space = 1 + 8 + 4 + (MAX_QUEUE_SIZE * ForcedWithdrawMessageInfo::LEN);
        let layer_zero_buffer_space = 1 + 8 + 4 + (MAX_QUEUE_SIZE * 10);
        let execution_buffer_space = 1 + 4 + (MAX_QUEUE_SIZE * ForcedWithdrawMessageInfo::LEN);
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut twine_chain_storage_lamports = rent.minimum_balance(twine_chain_storage_space);
        let mut current_batch_lamports = rent.minimum_balance(current_batch_space);
        let mut deposit_buffer_lamports = rent.minimum_balance(deposit_buffer_space);
        let mut withdraw_buffer_lamports = rent.minimum_balance(withdraw_buffer_space);
        let mut layer_zero_buffer_lamports = rent.minimum_balance(layer_zero_buffer_space);
        let mut execution_buffer_lamports = rent.minimum_balance(execution_buffer_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut twine_operation_handler_lamports = 1_000_000_000;

        // Setup Twine Chain Storage initial data
        let batch_info = BatchInfo {
            start_block: 0,
            end_block: 0,
        };

        let committed_info = BatchInfo {
            start_block: 1,
            end_block: 3,
        };

        let twine_chain_data = TwineChainStorage {
            is_initialized: true,
            groth16_vk: Vec::new(),
            execution_vkey: String::from(""),
            inclusion_vkey: String::from(""),
            withdrawal_vkey: String::from(""),
            skip_verification: true,
            last_finalized_batch: committed_info.clone(),
            last_committed_batch: committed_info.clone(),
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

        // Setup current batch account data
        let block1_info = BlockInfo {
            previous_hash: [0u8; 32],
            block_hash: [1u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [1u8; 32],
        };
        let block2_info = BlockInfo {
            previous_hash: [1u8; 32],
            block_hash: [2u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [2u8; 32],
        };
        let block3_info = BlockInfo {
            previous_hash: [2u8; 32],
            block_hash: [3u8; 32],
            transaction_root: [0u8; 32],
            receipt_root: [3u8; 32],
        };
        let batch_data = vec![block1_info, block2_info, block3_info];

        let batch_data_curr = BatchPdaAccount {
            is_initialized: true,
            infos: batch_data.clone(),
            verified: true,
            is_full: true,
        };
        let mut current_batch_data = vec![];
        batch_data_curr.serialize(&mut current_batch_data)?;

        // Setup deposit buffer data
        let deposit_message1 = DepositMessageInfo {
            nonce: 0,
            chain_id: 900,
            slot_number: 200,
            from_l1_pubkey: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            to_twine_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            l1_token: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            l2_token: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            amount: "1000000000000000000".to_string(),
        };
        let deposit_message2 = DepositMessageInfo {
            nonce: 1,
            chain_id: 900,
            slot_number: 200,
            from_l1_pubkey: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            to_twine_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            l1_token: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            l2_token: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            amount: "1000000000000000000".to_string(),
        };
        let deposit_buffer = DepositMessagesBuffer {
            is_initialized: true,
            deposit_nonce: 2,
            deposit_messages: vec![deposit_message1.clone(), deposit_message2.clone()],
        };
        let mut deposit_buffer_data = vec![];
        deposit_buffer.serialize(&mut deposit_buffer_data)?;

        // Setup withdraw buffer data
        let withdraw_message = ForcedWithdrawMessageInfo {
            nonce: 0,
            chain_id: 100,
            slot_number: 200,
            to_l1_pubkey: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            from_twine_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            l1_token: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            l2_token: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            amount: "1000000000000000000".to_string(),
        };
        let withdraw_buffer = ForcedWithdrawMessagesBuffer {
            is_initialized: true,
            withdraw_nonce: 1,
            withdraw_messages: vec![withdraw_message.clone()],
        };
        let mut withdraw_buffer_data = vec![];
        withdraw_buffer.serialize(&mut withdraw_buffer_data)?;

        // Setup layer zero buffer data
        let layer_zero_buffer = LayerZeroMessagesBuffer {
            is_initialized: true,
            lz_nonce: 0,
            lz_messages: vec![],
        };
        let mut layer_zero_buffer_data = vec![];
        layer_zero_buffer.serialize(&mut layer_zero_buffer_data)?;

        // Setup execution buffer data
        let execution_buffer = ExecutionMessageBuffer {
            is_initialized: true,
            withdrawals: vec![],
        };
        let mut execution_buffer_data = vec![0u8; execution_buffer_space];

        let mut temp = vec![];
        execution_buffer.serialize(&mut temp)?;
        execution_buffer_data[..temp.len()].copy_from_slice(&temp);

        // Setup remaining account's data
        let mut twine_operation_handler_data = vec![];

        // Setup owners
        let mut twine_chain_storage_owner = program_id;
        let mut current_batch_owner = program_id;
        let mut deposit_buffer_owner = program_id;
        let mut withdraw_buffer_owner = program_id;
        let mut layer_zero_buffer_owner = program_id;
        let mut execution_buffer_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut twine_operation_handler_owner = system_program_id;

        // Create required account infos
        let twine_chain_storage_account = create_test_account_info(
            &twine_chain_storage_key,
            false,
            true,
            &mut twine_chain_storage_lamports,
            &mut twine_chain_storage_data,
            &mut twine_chain_storage_owner,
        );

        let current_batch_account = create_test_account_info(
            &current_batch_key,
            false,
            true,
            &mut current_batch_lamports,
            &mut current_batch_data,
            &mut current_batch_owner,
        );

        let deposit_buffer_account = create_test_account_info(
            &deposit_buffer_key,
            false,
            true,
            &mut deposit_buffer_lamports,
            &mut deposit_buffer_data,
            &mut deposit_buffer_owner,
        );

        let withdraw_buffer_account = create_test_account_info(
            &withdraw_buffer_key,
            false,
            true,
            &mut withdraw_buffer_lamports,
            &mut withdraw_buffer_data,
            &mut withdraw_buffer_owner,
        );

        let layer_zero_buffer_account = create_test_account_info(
            &layer_zero_buffer_key,
            false,
            true,
            &mut layer_zero_buffer_lamports,
            &mut layer_zero_buffer_data,
            &mut layer_zero_buffer_owner,
        );

        let execution_buffer_account = create_test_account_info(
            &execution_buffer_key,
            false,
            true,
            &mut execution_buffer_lamports,
            &mut execution_buffer_data,
            &mut execution_buffer_owner,
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

        // Create accounts array in the correct order matching the function
        let accounts = vec![
            twine_chain_storage_account.clone(),
            current_batch_account.clone(),
            deposit_buffer_account.clone(),
            withdraw_buffer_account.clone(),
            layer_zero_buffer_account.clone(),
            execution_buffer_account.clone(),
            role_manager_account.clone(),
            twine_operation_handler_account.clone(),
        ];

        // Prepare Transaction Info
        let start_block: u64 = 1;
        let end_block: u64 = 3;
        let combined_receipt_root = calculate_combined_receipt_root(&batch_data);
        let selected_deposits = [deposit_message1, deposit_message2];
        let selected_withdrawals = [withdraw_message];
        let chain_commitment = ChainCommitment {
            deposit_count: 2,
            deposit_rolling_hash: calculate_deposit_rolling_hash(&selected_deposits),
            withdraw_count: 1,
            withdraw_rolling_hash: calculate_withdraw_rolling_hash(&selected_withdrawals),
            lz_transaction_count: 0,
            lz_transaction_rolling_hash: [0u8; 32],
        };

        let mut encoded_chain_commitment = Vec::with_capacity(120);
        encoded_chain_commitment.extend_from_slice(&chain_commitment.deposit_count.to_be_bytes());
        encoded_chain_commitment.extend_from_slice(&chain_commitment.deposit_rolling_hash);
        encoded_chain_commitment.extend_from_slice(&chain_commitment.withdraw_count.to_be_bytes());
        encoded_chain_commitment.extend_from_slice(&chain_commitment.withdraw_rolling_hash);
        encoded_chain_commitment
            .extend_from_slice(&chain_commitment.lz_transaction_count.to_be_bytes());
        encoded_chain_commitment.extend_from_slice(&chain_commitment.lz_transaction_rolling_hash);

        let mut transaction_info = vec![0u8; 288];
        transaction_info[0..8].copy_from_slice(&start_block.to_be_bytes());
        transaction_info[8..16].copy_from_slice(&end_block.to_be_bytes());
        transaction_info[16..48].copy_from_slice(&combined_receipt_root);
        transaction_info[168..288].copy_from_slice(&encoded_chain_commitment);

        // Call finalize Batch
        let result = commit_and_finalize_transaction(
            &program_id,
            &accounts,
            transaction_info.clone(),
            transaction_info,
        );
        assert!(
            result.is_ok(),
            "Transaction finalization Failed: {:?}",
            result.err()
        );

        // Verify Finalization
        let twine_chain_storage_data =
            TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data.borrow()[..])?;

        assert_eq!(
            twine_chain_storage_data
                .last_transaction_finalized_batch
                .start_block,
            1,
            "Last batch's start block should be 1"
        );

        assert_eq!(
            twine_chain_storage_data
                .last_transaction_finalized_batch
                .end_block,
            3,
            "Last batch's end block should be 3"
        );

        Ok(())
    }
}
