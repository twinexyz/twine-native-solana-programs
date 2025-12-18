use std::str::FromStr;

use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use oapp::{
    core::{instruction as oapp_instructions},
};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

pub fn initialize_store() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let endpoint_id = Pubkey::from_str("76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6").unwrap();

    let instructions = oapp_instructions::initialize_store(&endpoint_id, &account.pubkey());

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ Store Initialization Successful!");
    println!("Transaction: {}", signature);

    Ok(())
}
