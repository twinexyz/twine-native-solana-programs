use solana_program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use twine_chain::{
    core::{
        instruction::{self},
        state::BlockInfo,
    },
    id,
};

use crate::helpers::twine_chain_helper::{
    fund_account_for_rent_exemption, program_test, TwineChainAccounts,
};

mod helpers;

#[tokio::test]

async fn commit_batch_test() {
    let mut context = program_test().start_with_context().await;
    let accounts = TwineChainAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        BlockInfo::LEN,
        1_000_000_000,
    )
    .await;

    let mut instructions = vec![];

    instructions.extend(instruction::initialize_role_manager(
        &accounts.chain_admin.pubkey(),
    ));
    instructions.extend(instruction::initialize_twine_chain_storage(
        &accounts.chain_admin.pubkey(),
    ));
    instructions.extend(instruction::initialize_message_buffer(
        &accounts.chain_admin.pubkey(),
    ));

    
}
