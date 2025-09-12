use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tokens_gateway::{
    core::{instruction as tokens_gateway_instruction},
    id as tokens_gateway_id,
    utils::{
        address_derivation::{derive_native_token_vault_data,derive_spl_tokens_vault_data}
},
};
use twine_chain::core::{instruction as twine_chain_instruction,state::RoleType};

pub fn initialize_twine_solana_programs() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tokens_gateway_instructions = vec![];
    let native_token_valut_data_account = derive_native_token_vault_data(&tokens_gateway_id()).0;
    let spl_token_vault_data_account = derive_spl_tokens_vault_data(&tokens_gateway_id()).0;
    tokens_gateway_instructions.extend(
        tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&account.pubkey()),
    );

    tokens_gateway_instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &account.pubkey(),
    ));

    let tokens_gateway_initialize_transaction = Transaction::new_signed_with_payer(
        &tokens_gateway_instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let tokens_gateway_initialize_signature = rpc_client
        .send_and_confirm_transaction(&tokens_gateway_initialize_transaction)
        .context("Failed to send and confirm transaction")?;
    println!(
        "Transaction signature: {}",
        tokens_gateway_initialize_signature
    );
    print!("✅ Tokens Gateway Initialized");

    let mut twine_chain_instructions = vec![];
    twine_chain_instructions
        .extend(twine_chain_instruction::initialize_twine_chain_role_manager(&account.pubkey()));

    twine_chain_instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &account.pubkey(),
    ));

    twine_chain_instructions.extend(twine_chain_instruction::initialize_message_buffer(
        &account.pubkey(),
    ));

     twine_chain_instructions.extend(twine_chain_instruction::initialize_genesis_batch(
        &account.pubkey(),
        [0u8; 32]
    ));

    twine_chain_instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &account.pubkey(),
        &native_token_valut_data_account,
        RoleType::MessageAppender,

    ));
      twine_chain_instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &account.pubkey(),
        &spl_token_vault_data_account,
        RoleType::MessageAppender,

    ));

    let twine_chain_initialize_transaction = Transaction::new_signed_with_payer(
        &twine_chain_instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let twine_chain_initialize_signature = rpc_client
        .send_and_confirm_transaction(&twine_chain_initialize_transaction)
        .context("Failed to send and confirm transaction")?;
    println!();
    println!(
        "Transaction signature: {}",
        twine_chain_initialize_signature
    );
    print!("✅ Twine Chain Initialized");
    Ok(())
}
