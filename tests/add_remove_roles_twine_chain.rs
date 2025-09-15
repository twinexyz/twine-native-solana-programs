mod helpers;
use helpers::twine_chain_helper::{
    fund_account_for_rent_exemption, program_test, TwineChainAccounts,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use twine_chain::{
    core::{
        instruction as twine_chain_instruction,
        state::{
            RoleType,
        },
    },
    utils::{
        constants::MAX_ROLES,
    },
};

#[tokio::test]
async fn add_roles_twine_chain() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    let user_pubkey = Pubkey::from_str("EdhpXtonNKnVKpEK7iSzZvVU1gKSWtMjUaTuQZ5rvJkT").unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        1 + 32 + 4 + (MAX_ROLES * 33),
        1_000_000_000,
    )
    .await;
    let chain_admin = &accounts.chain_admin.pubkey();
    let mut instructions = vec![];
    instructions.extend(
        twine_chain_instruction::initialize_twine_chain_role_manager(
            &accounts.chain_admin.pubkey(),
        ),
    );
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
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
}
