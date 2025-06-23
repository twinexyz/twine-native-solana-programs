#![allow(clippy::arithmetic_side_effects)]

mod helpers;
use {
    helpers::tokens_gateway_helper::{
        create_spl_and_mint, fund_account_for_rent_exemption, get_or_create_ata, program_test,
        TokensGatewayAccounts,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    tokens_gateway::{
        core::instruction as tokens_gateway_instruction,
        utils::address_derivation::derive_spl_vault_authority,
        utils::constants::ROLE_MANAGER_ACCOUNT_SIZE, ID as tokens_gateway_ID,
    },
    twine_chain::core::instruction as twine_chain_instruction,
};

#[tokio::test]
async fn spl_withdrawal_finalized_succeed() {
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
    let amount = 2000000000u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let chain_id = 9u64;
    let block_number = 1u64;
    let nonce = 1u64;
    let is_forced_withdrawal = 1;
    let receipt_root: [u8; 32] = [
        0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
        0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
        0x1a, 0x1b,
    ];
    let inclusion_proof = vec![];
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let (spl_token_pubkey, user_token_account) =
        create_spl_and_mint(&mut context, &accounts.chain_admin, 9, 9000000000).await;
    let spl_token_vault = get_or_create_ata(
        &mut context,
        &accounts.chain_admin,
        &derive_spl_vault_authority(&tokens_gateway_ID).0,
        &spl_token_pubkey,
    )
    .await;
    let l1_token = spl_token_pubkey.to_string();

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
    instructions.extend(tokens_gateway_instruction::spl_token_deposit(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        &spl_token_vault,
        receiver_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        4000000000,
    ));
    instructions.extend(tokens_gateway_instruction::finalize_spl_token_withdrawal(
        chain_id,
        block_number,
        nonce,
        is_forced_withdrawal,
        receipt_root,
        chain_admin,
        &spl_token_pubkey,
        &spl_token_vault,
        user_token_account,
        l1_token.clone(),
        l2_token.clone(),
        amount.to_string(),
        inclusion_proof,
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
