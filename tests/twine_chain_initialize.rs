mod helpers;
use borsh::BorshDeserialize;
use solana_program_test::*;
use solana_sdk::{
    rent::Rent,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::twine_chain_helper::{
    fund_account_for_rent_exemption, program_test, TwineChainAccounts,
};
use twine_chain::{
    core::{
        instruction::{self},
        state::{BatchPdaAccount, MessagesBuffer, TwineChainRoleManager, TwineChainStorage},
    },
    id,
    utils::{
        address_derivation::{
            derive_commitment_pda, derive_messages_buffer, derive_twine_chain_storage,
        },
        constants::MAX_ROLES,
    },
};

#[tokio::test]

async fn twine_chain_rolemanager_init() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        1 + 32 + 4 + (MAX_ROLES * 33),
        1_000_000_000,
    )
    .await;

    let instructions =
        instruction::initialize_twine_chain_role_manager(&accounts.chain_admin.pubkey());
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );
    let error = context.banks_client.process_transaction(transaction).await;
    println!("Transaction Status: {:?}", error);

    let role_manager_account = context
        .banks_client
        .get_account(accounts.role_manager)
        .await
        .unwrap()
        .expect("RoleManager account not found");

    let role_manager_account_data: TwineChainRoleManager =
        TwineChainRoleManager::deserialize(&mut &role_manager_account.data[..])
            .expect("Failed to deserialize RoleManager");

    assert!(
        role_manager_account_data.is_initialized,
        "Rolemanager should be initialized"
    );

    assert_eq!(
        role_manager_account_data.chain_admin,
        accounts.chain_admin.pubkey(),
        "Chain admin should be set correctly"
    );
}

#[tokio::test]
async fn twine_chain_storage_init() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    let twine_chain_space = TwineChainStorage::LEN;
    let role_manager_space = 1 + 32 + 4 + (MAX_ROLES * 33);

    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        twine_chain_space + role_manager_space,
        1_000_000_000,
    )
    .await;

    let mut instructions = vec![];
    instructions.extend(instruction::initialize_twine_chain_role_manager(
        &accounts.chain_admin.pubkey(),
    ));
    instructions.extend(instruction::initialize_twine_chain_storage(
        &accounts.chain_admin.pubkey(),
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );
    let error = context.banks_client.process_transaction(transaction).await;
    println!("Transaction Status: {:?}", error);

    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let twine_chain_storage_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize RoleManager");

    assert!(
        twine_chain_storage_data.is_initialized,
        "Storage should be initialized"
    );
}

#[tokio::test]
async fn message_buffers_init() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    let rent = Rent::default();
    let role_manager_space = 1 + 32 + 4 + (MAX_ROLES * 33);

    let total_space = role_manager_space + 4 * 10240;
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        total_space,
        rent.minimum_balance(total_space),
    )
    .await;

    let mut instructions = vec![];
    instructions.extend(instruction::initialize_twine_chain_role_manager(
        &accounts.chain_admin.pubkey(),
    ));
    instructions.extend(instruction::initialize_message_buffer(
        &accounts.chain_admin.pubkey(),
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );
    let error = context.banks_client.process_transaction(transaction).await;
    println!("Transaction Status: {:?}", error);

    let messages_buffer_account = context
        .banks_client
        .get_account(derive_messages_buffer(&id()).0)
        .await
        .unwrap()
        .expect("Derived Messages Buffer account not found");

    let messages_buffer_data = MessagesBuffer::deserialize(&mut &messages_buffer_account.data[..])
        .expect("Failed to deserialize Deposit Buffer");

    assert!(
        messages_buffer_data.is_initialized,
        "Deposit Buffer should be initialized"
    );
}

#[tokio::test]

async fn genesis_batch_init() {
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

    let mut instructions = vec![];
    instructions.extend(instruction::initialize_twine_chain_role_manager(
        &accounts.chain_admin.pubkey(),
    ));
    instructions.extend(instruction::initialize_twine_chain_storage(
        &accounts.chain_admin.pubkey(),
    ));
    instructions.extend(instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        [1u8; 32],
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;
    println!("Transaction Status: {:?}", error);

    let genesis_batch_account = context
        .banks_client
        .get_account(derive_commitment_pda(&id(), 0u64).0)
        .await
        .unwrap()
        .expect("Genesis Batch storage account not found");

    let genesis_batch_data = BatchPdaAccount::deserialize(&mut &genesis_batch_account.data[..])
        .expect("Failed to deserialize Genesis Batch");

    assert!(
        genesis_batch_data.is_initialized,
        "Genesis Batch should be initialized"
    );

    assert_eq!(
        genesis_batch_data.batch_hash, [1u8; 32],
        "Batch hash should be set"
    )
}
