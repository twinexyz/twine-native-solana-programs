use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use oapp::core::instruction as oapp_instructions;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use std::str::FromStr;

pub fn init_config() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let endpoint_id = Pubkey::from_str("76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6").unwrap();
    let send_lib = Pubkey::from_str("7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH").unwrap();

    let instructions = oapp_instructions::init_config(&endpoint_id, &account.pubkey(), &send_lib);

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
