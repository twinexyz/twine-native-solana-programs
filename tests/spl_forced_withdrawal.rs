#[cfg(test)]
mod helpers;
#[path = "../scripts/interaction/twine_chain_client.rs"]
mod twine_chain_client;
use twine_chain_client as twine_chain_instruction;
#[path = "../scripts/interaction/tokens_gateway_client.rs"]
mod tokens_gateway_client;
use borsh::BorshDeserialize;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokens_gateway_client as tokens_gateway_instruction;

use helpers::tokens_gateway_helper::{
    create_spl_and_mint, fund_account_for_rent_exemption, get_ethereum_signature, program_test,
    TokensGatewayAccounts,
};
use tokens_gateway::{
    core::state::SignMessageInfo,
    id as tokens_gateway_id,
    utils::{
        address_derivation::derive_spl_tokens_vault_data, constants::ROLE_MANAGER_ACCOUNT_SIZE,
    },
};
use twine_chain::{
    core::state::{MessagesBuffer, RoleType, TwineChainStorage},
    id as twine_chain_id,
    utils::{
        address_derivation::{derive_messages_buffer, derive_twine_chain_storage},
        constants::MESSAGE_NONCE_GAP,
    },
};

#[tokio::test]
async fn spl_forced_withdrawal_succeed() {
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
    let l2_token = "0x1234567890abcdef1234567890abcdef12345678".to_string();
    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 2u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let from_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let (spl_token_pubkey, user_token_account) =
        create_spl_and_mint(&mut context, &accounts.chain_admin, 9, 100).await;
    let l1_token = spl_token_pubkey.to_string();
    let sign_info = SignMessageInfo {
        nonce: 1,
        chain_id: 900,
        amount: amount,
        l1_pubkey: user_token_account.to_string(),
        twine_address: from_twine_address.to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };
    let privkey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    let signature = get_ethereum_signature(&sign_info, privkey);
    let spl_token_valut_data_account = derive_spl_tokens_vault_data(&tokens_gateway_id()).0;

    let mut instructions = vec![];
    instructions.extend(twine_chain_instruction::initialize_twine_chain_role_manager(&chain_admin));
    instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &chain_admin,
    ));
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
        &spl_token_valut_data_account,
        RoleType::MessageAppender,
    ));
    instructions.extend(twine_chain_instruction::initialize_message_buffer(
        &chain_admin,
    ));
    instructions
        .extend(tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin));
    instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &chain_admin,
    ));
    instructions.extend(tokens_gateway_instruction::update_gateway_token_mapping(
        l1_token.clone(),
        l2_token.clone(),
        l1_decimals,
        l2_decimals,
        &chain_admin,
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);

    let mut other_instructions = vec![];
    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Twine Storage Account Not Found");

    let twine_chain_storage_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize TwineChainStorage");
    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;
    other_instructions.extend(tokens_gateway_instruction::forced_spl_token_withdrawal(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        from_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        start_nonce,
        end_nonce,
        signature,
    ));

    let other_transaction = Transaction::new_signed_with_payer(
        &other_instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let second_error = context
        .banks_client
        .process_transaction(other_transaction)
        .await;

    println!("ForcedTransaction status: {:?}", second_error);

    let forced_withdraw_message_buffer_account = context
        .banks_client
        .get_account(derive_messages_buffer(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Forced Message Buffer Not Found");

    let forced_withdraw_message_buffer_data: MessagesBuffer =
        MessagesBuffer::deserialize(&mut &forced_withdraw_message_buffer_account.data[..])
            .expect("Failed to deserialize Native Token Vault Data");
    assert!(
        forced_withdraw_message_buffer_data.message_nonce == 1,
        "Forced Withdrawal Not successful"
    )
}
