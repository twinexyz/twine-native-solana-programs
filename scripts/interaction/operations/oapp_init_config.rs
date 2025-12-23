use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use oapp::core::instruction as oapp_instructions;
use solana_sdk::{signature::Signer, transaction::Transaction};

pub fn init_config() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions = oapp_instructions::init_config(&account.pubkey());

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ Send Library Initialization Successful!");
    println!("Transaction: {}", signature);

    Ok(())
}
