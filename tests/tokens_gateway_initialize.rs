mod helpers;
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
    fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
};
use tokens_gateway::{
    core::state::{NativeTokenVaultData, TokensGatewayRoleManager},
    id as tokens_gateway_id,
    utils::address_derivation::{derive_gateway_role_manager, derive_native_token_vault_data},
    utils::constants::ROLE_MANAGER_ACCOUNT_SIZE,
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

    let tokens_gateway_rolemanager_account = context
        .banks_client
        .get_account(derive_gateway_role_manager(&tokens_gateway_id()).0)
        .await
        .unwrap()
        .expect("Twine chain storage account not found");

    let tokens_gateway_rolemanager_data: TokensGatewayRoleManager =
        TokensGatewayRoleManager::deserialize(&mut &tokens_gateway_rolemanager_account.data[..])
            .expect("Failed to deserialize RoleManager");
    assert!(
        tokens_gateway_rolemanager_data.is_initialized,
        "Rolemanager should be initialized"
    );
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
 let mut instructions = tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&accounts.chain_admin.pubkey());
    instructions.extend(
        tokens_gateway_instruction::initialize_tokens_gateway(&accounts.chain_admin.pubkey()));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );
    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
    let native_token_vault_account = context
        .banks_client
        .get_account(derive_native_token_vault_data(&tokens_gateway_id()).0)
        .await
        .unwrap()
        .expect("Native Token Vault Data Account Not Found");

    let native_token_vault_data: NativeTokenVaultData =
        NativeTokenVaultData::deserialize(&mut &native_token_vault_account.data[..])
            .expect("Failed to deserialize Native Token Vault Data");
    assert!(
        native_token_vault_data.is_initialized,
        "Native Token Vault should be initialized"
    );
}
