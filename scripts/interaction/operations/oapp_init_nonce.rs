use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use hex::FromHex;
use oapp::core::instruction as oapp_instructions;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use std::str::FromStr;

pub fn init_nonce(remote_oapp: String) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let endpoint_id = Pubkey::from_str("76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6").unwrap();

    let remote_oapp_bytes32 = evm_address_to_bytes32(remote_oapp);
    let instructions =
        oapp_instructions::init_nonce(&endpoint_id, &account.pubkey(), remote_oapp_bytes32);

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

fn evm_address_to_bytes32(addr: String) -> [u8; 32] {
    let cleaned = addr.trim_start_matches("0x");
    let raw_bytes: [u8; 20] =
        <[u8; 20]>::from_hex(cleaned).expect("Invalid EVM address hex string");

    // Left-pad the 20 bytes to 32 bytes
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(&raw_bytes);
    padded
}
