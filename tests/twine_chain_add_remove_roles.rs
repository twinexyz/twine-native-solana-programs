mod helpers;
#[path = "../scripts/interaction/twine_chain_client.rs"]
mod twine_chain_client;
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use twine_chain_client as twine_chain_instruction;

use helpers::twine_chain_helper::{
    fund_account_for_rent_exemption, program_test, TwineChainAccounts,
};

use tokens_gateway::utils::constants::ROLE_MANAGER_ACCOUNT_SIZE;

use twine_chain::{
    core::state::{RoleType, TwineChainRoleManager},
    id as twine_chain_id,
    utils::address_derivation::derive_twine_chain_role_manager,
};

#[tokio::test]
async fn add_roles_twine_chain() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
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
    let mut instructions = vec![];
    instructions.extend(
        twine_chain_instruction::initialize_twine_chain_role_manager(
            &accounts.chain_admin.pubkey(),
        ),
    );
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &accounts.chain_admin.pubkey(),
        &user_pubkey,
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

    let twine_chain_role_manager_account = context
        .banks_client
        .get_account(derive_twine_chain_role_manager(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Role manager Account Not Found");
    let twine_chain_role_manager_data: TwineChainRoleManager =
        TwineChainRoleManager::deserialize(&mut &twine_chain_role_manager_account.data[..])
            .expect("Failed to deserialize Rolemanager Data");
    let has_role = twine_chain_role_manager_data
        .roles
        .iter()
        .any(|(pk, role)| pk == &user_pubkey && *role == expected_role);
    assert!(has_role, "User does not have the expected role");
}

#[tokio::test]
async fn remove_roles_twine_chain() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
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
    let mut instructions = vec![];
    instructions.extend(
        twine_chain_instruction::initialize_twine_chain_role_manager(
            &accounts.chain_admin.pubkey(),
        ),
    );
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &accounts.chain_admin.pubkey(),
        &user_pubkey,
        RoleType::TwineOperationHandler,
    ));
    instructions.extend(twine_chain_instruction::remove_role_in_twine_chain(
        &accounts.chain_admin.pubkey(),
        &user_pubkey,
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
    let twine_chain_role_manager_account = context
        .banks_client
        .get_account(derive_twine_chain_role_manager(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Role manager Account Not Found");
    let twine_chain_role_manager_data: TwineChainRoleManager =
        TwineChainRoleManager::deserialize(&mut &twine_chain_role_manager_account.data[..])
            .expect("Failed to deserialize Twine chain Rolemanager Data");
    let has_role = twine_chain_role_manager_data
        .roles
        .iter()
        .any(|(pk, role)| pk == &user_pubkey && *role == expected_role);
    assert!(!has_role, "User have the expected role");
}
