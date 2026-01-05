#[cfg(test)]
mod helpers;
#[path = "../scripts/interaction/twine_chain_client.rs"]
mod twine_chain_client;
use twine_chain_client as twine_chain_instruction;
#[path = "../scripts/interaction/tokens_gateway_client.rs"]
mod tokens_gateway_client;
use tokens_gateway_client as tokens_gateway_instruction;

use borsh::BorshDeserialize;
use helpers::tokens_gateway_helper::{
    fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokens_gateway::{
    id as tokens_gateway_id, utils::address_derivation::derive_native_token_vault_data,
    utils::constants::ROLE_MANAGER_ACCOUNT_SIZE,
};
use twine_chain::{
    core::state::{DetailedMessagesBuffer, MessagesReplicator, RoleType, TwineChainStorage},
    id as twine_chain_id,
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_messages_replicator, derive_twine_chain_storage,
        },
        constants::MESSAGE_NONCE_GAP,
    },
};

#[tokio::test]
async fn copy_messages_buffer() {
    let mut context = program_test().start_with_context().await;
    let accounts = TokensGatewayAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();

    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        ROLE_MANAGER_ACCOUNT_SIZE,
        844073716442015,
    )
    .await;
    let l1_token = "11111111111111111111111111111111".to_string();
    let l2_token = "0x1234567890abcdef1234567890abcdef12345678".to_string();
    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 1u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let data = "".to_string();
    let native_token_valut_data_account = derive_native_token_vault_data(&tokens_gateway_id()).0;
    let run_time = 300u64;
    let mut start_nonce = 1u64;
    let mut end_nonce = MESSAGE_NONCE_GAP;
    let mut current_nonce = 0u64;
    let initial_start_nonce = start_nonce;
    let initial_end_nonce = end_nonce;

    // One-time program setup
    let mut init_instructions = vec![];
    init_instructions.extend(twine_chain_instruction::initialize_twine_chain_role_manager(
        &chain_admin,
    ));
    init_instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &chain_admin,
    ));
    init_instructions.extend(twine_chain_instruction::initialize_message_buffer(
        &chain_admin,
    ));
    init_instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
        &native_token_valut_data_account,
        RoleType::MessageAppender,
    ));
    init_instructions.extend(
        tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin),
    );
    init_instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &chain_admin,
    ));
    init_instructions.extend(tokens_gateway_instruction::update_gateway_token_mapping(
        l1_token.clone(),
        l2_token.clone(),
        l1_decimals,
        l2_decimals,
        &chain_admin,
    ));
    let setup_tx = Transaction::new_signed_with_payer(
        &init_instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.banks_client.get_latest_blockhash().await.unwrap(),
    );
    context
        .banks_client
        .process_transaction(setup_tx)
        .await
        .expect("setup tx failed");

    // Send deposits in small batches to avoid trace/compute timeouts
    const BATCH_SIZE: usize = 10;
    let mut pending: Vec<solana_sdk::instruction::Instruction> = Vec::new();
    for _ in 0..run_time {
        let deposit_ix = tokens_gateway_instruction::native_token_deposit(
            &chain_admin,
            receiver_twine_address.clone(),
            l1_token.clone(),
            l2_token.clone(),
            amount,
            start_nonce,
            end_nonce,
            hex::decode(data.clone()).unwrap(),
        );
        pending.extend(deposit_ix);
        current_nonce += 1;
        if current_nonce > end_nonce {
            start_nonce = end_nonce + 1;
            end_nonce += MESSAGE_NONCE_GAP;
        }
        if pending.len() >= BATCH_SIZE {
            let deposit_tx = Transaction::new_signed_with_payer(
                &pending,
                Some(&context.payer.pubkey()),
                &[&context.payer, &accounts.chain_admin],
                context.banks_client.get_latest_blockhash().await.unwrap(),
            );
            let res = context.banks_client.process_transaction(deposit_tx).await;
            assert!(res.is_ok(), "Deposit batch failed: {:?}", res);
            pending.clear();
        }
    }
    if !pending.is_empty() {
        let deposit_tx = Transaction::new_signed_with_payer(
            &pending,
            Some(&context.payer.pubkey()),
            &[&context.payer, &accounts.chain_admin],
            context.banks_client.get_latest_blockhash().await.unwrap(),
        );
        let res = context.banks_client.process_transaction(deposit_tx).await;
        assert!(res.is_ok(), "Final deposit batch failed: {:?}", res);
    }
    let deposit_message_buffer_account = context
        .banks_client
        .get_account(derive_detailed_messages_buffer(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Deposit Message Account Not Found");
    let deposit_message_buffer_data: DetailedMessagesBuffer =
        DetailedMessagesBuffer::deserialize(&mut &deposit_message_buffer_account.data[..])
            .expect("Failed to deserialize Deposit Message Buffer Data");

    println!("Message Buffer Data: {:?}", deposit_message_buffer_data);
    assert_eq!(
        deposit_message_buffer_data.messages.len(),
        (run_time - MESSAGE_NONCE_GAP) as usize,
        "Detailed buffer should keep only the messages beyond the copied range"
    );
    assert!(
        deposit_message_buffer_data.message_nonce >= run_time,
        "Message nonce should reflect all appended messages"
    );

    let message_replicator_account = context
        .banks_client
        .get_account(
            derive_messages_replicator(&twine_chain_id(), initial_start_nonce, initial_end_nonce).0,
        )
        .await
        .unwrap()
        .expect("Messages Replicator Not Found");
    let message_replicator_data: MessagesReplicator =
        MessagesReplicator::deserialize(&mut &message_replicator_account.data[..])
            .expect("Failed to deserialize Message Replicator Data");

    println!("Message Replicator Data: {:?}", message_replicator_data);
    assert!(message_replicator_data.is_initialized, "Replicator must be initialized");
    assert_eq!(
        message_replicator_data.start_nonce, initial_start_nonce,
        "Replicator start_nonce should match the first copied nonce"
    );
    assert_eq!(
        message_replicator_data.end_nonce, initial_end_nonce,
        "Replicator end_nonce should match the last copied nonce"
    );
    assert_eq!(
        message_replicator_data.messages.len(),
        MESSAGE_NONCE_GAP as usize,
        "Replicator should contain exactly MESSAGE_NONCE_GAP messages"
    );

    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Twine Chain Storage Not Found");
    let twine_chain_storage_account_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize Twine Storage Data");

    println!("Twine Storage Data: {:?}", twine_chain_storage_account_data);

    assert_eq!(
        twine_chain_storage_account_data.last_copied_message_end_nonce,
        MESSAGE_NONCE_GAP,
        "last_copied_message_end_nonce should match MESSAGE_NONCE_GAP"
    );
 
}
