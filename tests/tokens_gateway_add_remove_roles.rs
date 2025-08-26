mod helpers;
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

use helpers::tokens_gateway_helper::{
    fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
};

use tokens_gateway::{
    core::{
        instruction as tokens_gateway_instruction,
        state::{RoleType, TokensGatewayRoleManager},
    },
    id as tokens_gateway_id,
    utils::address_derivation::derive_gateway_role_manager,
    utils::constants::ROLE_MANAGER_ACCOUNT_SIZE,
};

#[tokio::test]
async fn add_roles_tokens_gateway() {
    let mut context = program_test().start_with_context().await;
    let accounts = TokensGatewayAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    let user_pubkey = Pubkey::from_str("EdhpXtonNKnVKpEK7iSzZvVU1gKSWtMjUaTuQZ5rvJkT").unwrap();
    let expected_role = RoleType::TwineOperationHandler;
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        ROLE_MANAGER_ACCOUNT_SIZE,
        844073716442015,
    )
    .await;
    let chain_admin = &accounts.chain_admin.pubkey();
    let mut instructions = vec![];
    instructions
        .extend(tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin));
    instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &chain_admin,
    ));
    instructions.extend(tokens_gateway_instruction::add_role_in_gateway(
        user_pubkey,
        accounts.chain_admin.pubkey(),
        RoleType::TwineOperationHandler,
    ));
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);

    let tokens_gateway_role_manager_account = context
        .banks_client
        .get_account(derive_gateway_role_manager(&tokens_gateway_id()).0)
        .await
        .unwrap()
        .expect("Deposit Message Account Not Found");
    let tokens_gateway_role_manager_data: TokensGatewayRoleManager =
        TokensGatewayRoleManager::deserialize(&mut &tokens_gateway_role_manager_account.data[..])
            .expect("Failed to deserialize Tokens Gateway Rolemanager Data");
    let has_role = tokens_gateway_role_manager_data
        .roles
        .iter()
        .any(|(pk, role)| pk == &user_pubkey && *role == expected_role);
    assert!(has_role, "User does not have the expected role");
}

#[tokio::test]
async fn remove_roles_tokens_gateway() {
    let mut context = program_test().start_with_context().await;
    let accounts = TokensGatewayAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    let user_pubkey = Pubkey::from_str("EdhpXtonNKnVKpEK7iSzZvVU1gKSWtMjUaTuQZ5rvJkT").unwrap();
    let expected_role = RoleType::TwineOperationHandler;
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        ROLE_MANAGER_ACCOUNT_SIZE,
        844073716442015,
    )
    .await;
    let chain_admin = &accounts.chain_admin.pubkey();
    let mut instructions = vec![];
    instructions
        .extend(tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin));
    instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &chain_admin,
    ));
    instructions.extend(tokens_gateway_instruction::add_role_in_gateway(
        user_pubkey,
        RoleType::TwineOperationHandler,
        accounts.chain_admin.pubkey(),
    ));
    instructions.extend(tokens_gateway_instruction::remove_role_in_gateway(
        user_pubkey,
        RoleType::TwineOperationHandler,
        accounts.chain_admin.pubkey(),
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
    let tokens_gateway_role_manager_account = context
        .banks_client
        .get_account(derive_gateway_role_manager(&tokens_gateway_id()).0)
        .await
        .unwrap()
        .expect("Deposit Message Account Not Found");
    let tokens_gateway_role_manager_data: TokensGatewayRoleManager =
        TokensGatewayRoleManager::deserialize(&mut &tokens_gateway_role_manager_account.data[..])
            .expect("Failed to deserialize Tokens Gateway Rolemanager Data");
    let has_role = tokens_gateway_role_manager_data
        .roles
        .iter()
        .any(|(pk, role)| pk == &user_pubkey && *role == expected_role);
    assert!(!has_role, "User have the expected role");
}
