use crate::utils::{get_default_keypair, get_rpc_client};
use crate::tokens_gateway_client as tokens_gateway_instruction;
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};

pub fn update_token_mapping(
    l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions = tokens_gateway_instruction::update_gateway_token_mapping(
        l1_token,
        l2_token,
        l1_decimals,
        l2_decimals,
        &account.pubkey(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ Token Mapping Done!");
    println!("Transaction: {}", signature);
    Ok(())
}

pub fn remove_token_mapping(
    l1_token: String,
    l2_token: String,
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions = tokens_gateway_instruction::remove_gateway_token_mapping(
        l1_token,
        l2_token,
        &account.pubkey(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ Token Mapping Removed!");
    println!("Transaction: {}", signature);
    Ok(())
}