mod helpers;

use borsh::BorshDeserialize;
use sha3::{Digest, Keccak256};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use helpers::twine_chain_helper::{
    fund_account_for_rent_exemption, program_test, TwineChainAccounts,
};
use twine_chain::{
    core::{
        instruction::{self},
        state::{
            BlockInfo, ChainCommitment, CommitBatchInfo, DepositMessageInfo, DepositMessagesBuffer,
            ForcedWithdrawMessageInfo, TwineChainStorage,
        },
    },
    id,
    utils::address_derivation::{derive_deposit_message_buffer, derive_twine_chain_storage},
};

#[tokio::test]

async fn transaction_commitment_and_finalization_test() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        BlockInfo::LEN,
        1_000_000_000,
    )
    .await;

    //Instructions to be executed:
    let mut instructions = vec![];

    // 1. Initialize role manager
    instructions.extend(instruction::initialize_twine_chain_role_manager(
        &accounts.chain_admin.pubkey(),
    ));

    // 2. Initialize twine chain storage
    instructions.extend(instruction::initialize_twine_chain_storage(
        &accounts.chain_admin.pubkey(),
    ));

    // 3. Initialize message buffers
    instructions.extend(instruction::initialize_message_buffer(
        &accounts.chain_admin.pubkey(),
    ));

    // 4. Initialize Genesis Batch
    instructions.extend(instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        [0u8; 32],
    ));

    // 5. Populate Deposit Message Buffer:
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

    instructions.extend(instruction::append_deposit_message(
        &accounts.chain_admin.pubkey(),
        deposit_message1.clone(),
    ));
    instructions.extend(instruction::append_deposit_message(
        &accounts.chain_admin.pubkey(),
        deposit_message2.clone(),
    ));

    // 6. Populate Withdraw Message Buffer:
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

    instructions.extend(instruction::append_forced_withdrawal_message(
        &accounts.chain_admin.pubkey(),
        withdraw_message.clone(),
    ));

    // 7. Commit a Batch
    let start_block = 1;
    let end_block = 3;

    let first_block = CommitBatchInfo {
        block_number: 1,
        block_hash: [1u8; 32],
        transaction_root: [0u8; 32],
        receipt_root: [1u8; 32],
    };

    let second_block = CommitBatchInfo {
        block_number: 2,
        block_hash: [2u8; 32],
        transaction_root: [0u8; 32],
        receipt_root: [2u8; 32],
    };

    let third_block = CommitBatchInfo {
        block_number: 3,
        block_hash: [3u8; 32],
        transaction_root: [0u8; 32],
        receipt_root: [3u8; 32],
    };

    let commitment_batch_data = vec![first_block, second_block, third_block];

    instructions.extend(instruction::commit_batch(
        &accounts.chain_admin.pubkey(),
        start_block,
        end_block,
        commitment_batch_data,
    ));

    // 8. Finalize a batch
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
    let finalization_batch_data = vec![block1_info, block2_info, block3_info];

    let mut calculated_batch_hash = [0u8; 32];
    let mut serialized_batch_hash: Vec<u8> =
        Vec::with_capacity(BlockInfo::LEN * finalization_batch_data.len());

    for block in finalization_batch_data.clone() {
        serialized_batch_hash.extend_from_slice(&block.abi_encode_packed());
    }
    let mut hasher = Keccak256::new();
    hasher.update(serialized_batch_hash);

    let encoded_batch_hash = hasher.finalize().to_vec();
    calculated_batch_hash[..32].copy_from_slice(&encoded_batch_hash[..32]);

    let mut public_values = Vec::with_capacity(48);
    public_values.extend_from_slice(&start_block.to_be_bytes());
    public_values.extend_from_slice(&end_block.to_be_bytes());
    public_values.extend_from_slice(&calculated_batch_hash);

    instructions.extend(instruction::finalize_batch(
        &accounts.chain_admin.pubkey(),
        start_block,
        end_block,
        public_values.clone(),
        public_values,
    ));

    // 9. Commit and Finalize the transaction
    let start_block: u64 = 1;
    let end_block: u64 = 3;
    let combined_receipt_root = calculate_combined_receipt_root(&finalization_batch_data);
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

    instructions.extend(instruction::commit_and_finalize_transaction(
        &accounts.chain_admin.pubkey(),
        start_block,
        end_block,
        transaction_info.clone(),
        transaction_info,
    ));

    // Making the transaction:
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );
    let error = context.banks_client.process_transaction(transaction).await;
    println!("Transaction1 Status: {:?}", error);

    // Getting accounts
    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let deposit_buffer_account = context
        .banks_client
        .get_account(derive_deposit_message_buffer(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let twine_chain_storage_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize RoleManager");

    let deposit_buffer_data =
        DepositMessagesBuffer::deserialize(&mut &deposit_buffer_account.data[..])
            .expect("Failed to deserialize deposit buffer");

    // Checking for correct commitment and finalization
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
    assert_eq!(
        deposit_buffer_data.deposit_messages.len(),
        0,
        "Deposit Buffer should be empty"
    );
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
