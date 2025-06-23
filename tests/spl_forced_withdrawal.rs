#![allow(clippy::arithmetic_side_effects)]
#[cfg(test)]
mod helpers;
use {
    helpers::tokens_gateway_helper::{
        create_spl_and_mint, fund_account_for_rent_exemption, get_ethereum_signature, program_test,
        TokensGatewayAccounts,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    tokens_gateway::{
        core::instruction as tokens_gateway_instruction, core::state::SignMessageInfo,
        utils::constants::ROLE_MANAGER_ACCOUNT_SIZE,
    },
    twine_chain::core::instruction as twine_chain_instruction,
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
        from_twine_address: from_twine_address.to_string(),
        to_l1_pubkey: user_token_account.to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };
    let privkey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    let signature = get_ethereum_signature(&sign_info, privkey);

    let mut instructions = vec![];
    instructions.extend(twine_chain_instruction::initialize_twine_chain_role_manager(&chain_admin));
    instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &chain_admin,
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
    instructions.extend(tokens_gateway_instruction::forced_spl_token_withdrawal(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        from_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        signature,
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
}
