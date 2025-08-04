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
        state::{BatchPdaAccount, TwineChainStorage},
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
        BatchPdaAccount::LEN,
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
    let batch_number = 1;
    let batch_hash = [2u8; 32];

    instructions.extend(instruction::commit_batch(
        &accounts.chain_admin.pubkey(),
        batch_number,
        batch_hash,
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
        .get_account(derive_commitment_pda(&id(), batch_number).0)
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

    assert_eq!(
        twine_chain_storage_data.last_committed_batch_number, 1,
        "Last batch's start block should be 1"
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
        BatchPdaAccount::LEN,
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
    let genesis_block_hash = [0u8; 32];
    instructions.extend(instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        genesis_block_hash,
    ));

     // 4. Commit a Batch
    let batch_number = 1;
    let batch_hash = [5u8; 32];

    instructions.extend(instruction::commit_batch(
        &accounts.chain_admin.pubkey(),
        batch_number,
        batch_hash,
    ));

    // 5. Finalize a batch
    let total_msg_handled_on_twine : u64 = 2;

    let mut public_values = Vec::with_capacity(72);
    public_values.extend_from_slice(&genesis_block_hash);
    public_values.extend_from_slice(&batch_hash);
    public_values.extend_from_slice(&total_msg_handled_on_twine.to_be_bytes());
    public_values.extend_from_slice(&total_msg_handled_on_twine.to_be_bytes());

    instructions.extend(instruction::finalize_batch(
        &accounts.chain_admin.pubkey(),
        batch_number,
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
        .get_account(derive_commitment_pda(&id(), batch_number).0)
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

    assert_eq!(
        twine_chain_storage_data.last_finalized_batch_number, 1,
        "Last batch's start block should be 1"
    );

}
