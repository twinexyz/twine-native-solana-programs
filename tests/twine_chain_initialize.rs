mod helpers;

use {
    borsh::BorshDeserialize,
    helpers::twine_chain_helper::{
        fund_account_for_rent_exemption, program_test, TwineChainAccounts,
    },
    solana_program_test::*,
    solana_sdk::{
        rent::Rent,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    twine_chain::{
        core::{
            instruction::{self},
            state::{
                BatchPdaAccount, BlockInfo, DepositMessagesBuffer,
                ExecutionMessageBuffer, ForcedWithdrawMessagesBuffer, LayerZeroMessagesBuffer,
                TwineChainRoleManager, TwineChainStorage,
            },
        },
        id,
        utils::{
            address_derivation::{
                derive_commitment_pda, derive_deposit_message_buffer,
                derive_execution_message_buffer, derive_forced_withdraw_message_buffer,
                derive_layer_zero_message_buffer, derive_twine_chain_storage,
            },
            constants::MAX_ROLES,
        },
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

    let instructions = instruction::initialize_role_manager(&accounts.chain_admin.pubkey());
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
async fn twine_chain_storage_inti() {
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
    instructions.extend(instruction::initialize_role_manager(
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
    instructions.extend(instruction::initialize_role_manager(
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

    let deposit_buffer_account = context
        .banks_client
        .get_account(derive_deposit_message_buffer(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let forced_withdraw_buffer_account = context
        .banks_client
        .get_account(derive_forced_withdraw_message_buffer(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let layer_zero_buffer_account = context
        .banks_client
        .get_account(derive_layer_zero_message_buffer(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let execution_buffer_account = context
        .banks_client
        .get_account(derive_execution_message_buffer(&id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let deposit_buffer_data =
        DepositMessagesBuffer::deserialize(&mut &deposit_buffer_account.data[..])
            .expect("Failed to deserialize Deposit Buffer");

    let forced_withdraw_buffer_data =
        ForcedWithdrawMessagesBuffer::deserialize(&mut &forced_withdraw_buffer_account.data[..])
            .expect("Failed to deserialize Withdraw Buffer");

    let layer_zero_buffer_data =
        LayerZeroMessagesBuffer::deserialize(&mut &layer_zero_buffer_account.data[..])
            .expect("Failed to deserialize Execution Buffer");

    let execution_buffer_data =
        ExecutionMessageBuffer::deserialize(&mut &execution_buffer_account.data[..])
            .expect("Failed to deserialize Execution Buffer");

    assert!(
        deposit_buffer_data.is_initialized,
        "Deposit Buffer should be initialized"
    );

    assert!(
        forced_withdraw_buffer_data.is_initialized,
        "Withdraw Buffer should be initialized"
    );

    assert!(
        layer_zero_buffer_data.is_initialized,
        "LZ Buffer should be initialized"
    );

    assert!(
        execution_buffer_data.is_initialized,
        "Execution Buffer should be initialized"
    );
}

#[tokio::test]

async fn genesis_batch_inti() {
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

    let mut instructions = vec![];
    instructions.extend(instruction::initialize_role_manager(
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
    println!("Transaction1 Status: {:?}", error);

    let genesis_batch_account = context
        .banks_client
        .get_account(derive_commitment_pda(&id(), 0, 0).0)
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
        genesis_batch_data.infos[0].block_hash, [1u8; 32],
        "Batch hash should be set"
    )
}
