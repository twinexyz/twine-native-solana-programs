#![allow(clippy::arithmetic_side_effects)]
#[cfg(test)]
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
    core::{instruction as tokens_gateway_instruction, state::SignMessageInfo},
    id,
    utils::{
        address_derivation::derive_native_token_vault_data, constants::ROLE_MANAGER_ACCOUNT_SIZE,
    },
};
use twine_chain::core::{instruction as twine_chain_instruction, state::RoleType};

use crate::helpers::tokens_gateway_helper::get_ethereum_signature;

#[tokio::test]
async fn native_withdrawal_finalized_succeed() {
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
    let amount = 1000000000u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let l1_receiver_address = accounts.chain_admin.pubkey();
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();

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
    let native_token_valut_data_account = derive_native_token_vault_data(&id()).0;

    let sign_info = SignMessageInfo {
        nonce: 1,
        chain_id: 900,
        amount: amount,
        from_twine_address: receiver_twine_address.to_string(),
        to_l1_pubkey: chain_admin.to_string(),
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
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
        &native_token_valut_data_account,
        RoleType::MessageAppender,
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
    instructions.extend(tokens_gateway_instruction::native_token_deposit(
        &chain_admin,
        receiver_twine_address.clone(),
        l1_token.clone(),
        l2_token.clone(),
        2000000000,
        "".to_string(),
    ));

    instructions.extend(tokens_gateway_instruction::forced_native_token_withdrawal(
        &chain_admin,
        receiver_twine_address.clone(),
        chain_admin.to_string(),
        l1_token.clone(),
        l2_token.clone(),
        amount,
        signature,
    ));
    instructions.extend(
        tokens_gateway_instruction::finalize_native_token_withdrawal(
            chain_id,
            block_number,
            nonce,
            is_forced_withdrawal,
            receipt_root,
            l1_receiver_address,
            l1_token.clone(),
            l2_token.clone(),
            amount.to_string(),
            inclusion_proof,
        ),
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
