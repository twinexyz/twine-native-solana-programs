#![allow(clippy::arithmetic_side_effects)]

mod helpers;

use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::tokens_gateway_helper::{
    fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction, utils::constants::ROLE_MANAGER_ACCOUNT_SIZE,
};

#[tokio::test]
async fn tokens_gateway_rolemanager_init() {
    let mut context = program_test().start_with_context().await;
    let accounts = TokensGatewayAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        ROLE_MANAGER_ACCOUNT_SIZE,
        1_000_000,
    )
    .await;
    let instructions = tokens_gateway_instruction::initialize_tokens_gateway_role_manager(
        &accounts.chain_admin.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
}

#[tokio::test]
async fn tokens_gateway_init() {
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
    let instructions =
        tokens_gateway_instruction::initialize_tokens_gateway(&accounts.chain_admin.pubkey());

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );
    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
}
