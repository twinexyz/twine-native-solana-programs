use borsh::BorshDeserialize;
use sha3::{Digest, Keccak256};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
mod helpers;
use twine_chain::{
    core::{
        instruction::{self},
        state::{BatchPdaAccount, BlockInfo, CommitBatchInfo, TwineChainStorage},
    },
    id,
    utils::address_derivation::{derive_commitment_pda, derive_twine_chain_storage},
};

use helpers::twine_chain_helper::{
    fund_account_for_rent_exemption, program_test, TwineChainAccounts,
};


#[tokio::test]
async fn commit_batch_test() {
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

    // 3. Initialize Genesis Batch
    instructions.extend(instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        [1u8; 32],
    ));

    // 4. Commit a Batch
    let start_block = 1;
    let end_block = 3;

    let first_block = CommitBatchInfo {
        block_number: 1,
        block_hash: [2u8; 32],
        transaction_root: [0u8; 32],
        receipt_root: [1u8; 32],
    };

    let second_block = CommitBatchInfo {
        block_number: 2,
        block_hash: [3u8; 32],
        transaction_root: [0u8; 32],
        receipt_root: [2u8; 32],
    };

    let third_block = CommitBatchInfo {
        block_number: 3,
        block_hash: [4u8; 32],
        transaction_root: [0u8; 32],
        receipt_root: [3u8; 32],
    };

    let batch_data = vec![first_block, second_block, third_block];

    instructions.extend(instruction::commit_batch(
        &accounts.chain_admin.pubkey(),
        start_block,
        end_block,
        batch_data,
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
    let current_batch_account = context
        .banks_client
        .get_account(derive_commitment_pda(&id(), start_block, end_block).0)
        .await
        .unwrap()
        .expect("Current batch account not found");

    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let current_batch_data = BatchPdaAccount::deserialize(&mut &current_batch_account.data[..])
        .expect("Failed to deserialize Genesis Batch");

    let twine_chain_storage_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize RoleManager");

    // Checking for correct commitment
    assert!(
        current_batch_data.is_initialized,
        "Current batch should be initialized"
    );

    assert!(current_batch_data.is_full, "is_full should be set to true");

    assert_eq!(
        current_batch_data.infos.len(),
        3,
        "There should data for 3 blocks"
    );

    assert!(!current_batch_data.verified, "Batch should not be verified");

    assert_eq!(
        twine_chain_storage_data.last_committed_batch.start_block, 1,
        "Last batch's start block should be 1"
    );

    assert_eq!(
        twine_chain_storage_data.last_committed_batch.end_block, 3,
        "Last batch's end block should be 3"
    );
}

#[tokio::test]
async fn finalize_batch_test() {
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

    // 3. Initialize Genesis Batch
    instructions.extend(instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        [0u8; 32],
    ));

    // 4. Commit a Batch
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

    let batch_data = vec![first_block, second_block, third_block];

    instructions.extend(instruction::commit_batch(
        &accounts.chain_admin.pubkey(),
        start_block,
        end_block,
        batch_data,
    ));

    // 5. Finalize a batch
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

    let mut calculated_batch_hash = [0u8; 32];
    let mut serialized_batch_hash: Vec<u8> = Vec::with_capacity(BlockInfo::LEN * batch_data.len());

    for block in batch_data {
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
    let current_batch_account = context
        .banks_client
        .get_account(derive_commitment_pda(&id(), start_block, end_block).0)
        .await
        .unwrap()
        .expect("Current batch account not found");

    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let current_batch_data = BatchPdaAccount::deserialize(&mut &current_batch_account.data[..])
        .expect("Failed to deserialize Genesis Batch");

    let twine_chain_storage_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize RoleManager");

    // Checking for correct commitment
    assert!(current_batch_data.verified, "Batch should be verified");

    assert_eq!(
        twine_chain_storage_data.last_finalized_batch.start_block, 1,
        "Last batch's start block should be 1"
    );

    assert_eq!(
        twine_chain_storage_data.last_finalized_batch.end_block, 3,
        "Last batch's end block should be 3"
    );
}
