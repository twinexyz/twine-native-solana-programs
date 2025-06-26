#![allow(clippy::arithmetic_side_effects)]

use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tokens_gateway::core::instruction as tokens_gateway_instruction;

pub fn native_token_deposit(
    l1_token: String,
    l2_token: String,
    receiver_twine_address: String,
    amount: u64,
    data: String
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions = tokens_gateway_instruction::native_token_deposit(
        &account.pubkey(),
        receiver_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        data
    );

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;
    print!("Native token (SOL) deposited");
    Ok(())
}
