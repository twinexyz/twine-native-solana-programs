use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use borsh::BorshSerialize;
use hex::FromHex;
use oapp::core::{instruction as oapp_instructions, state::SendMsgParams};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use std::str::FromStr;

pub fn send_message() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let receiver_address = String::from("0x91607f93f3F46e05e62AE910FE0d75fB001E74b6");
    let dvn_account = Pubkey::from_str("A8BmGrQ7vuNNy5URzvfM5nr8RvWodAheMxjMcBQULiUE").unwrap();
    let executor_account =
        Pubkey::from_str("2DvDmd8SGrsuSdSrG15GgM4Ay3tNT49oE4cPEJRmkiXc").unwrap();

    let options_hex = "0x00030100110100000000000000000000000000000000";
    let options = hex::decode(options_hex.trim_start_matches("0x"))?;

    let params = SendMsgParams {
        dst_eid: 40161,
        receiver: evm_address_to_bytes32(receiver_address),
        message: "asdasd".try_to_vec()?,
        options: options,
        native_fee: 0,
        lz_token_fee: 0,
    };

    let instructions = oapp_instructions::send_message(
        params,
        &account.pubkey(),
        &dvn_account,
        &executor_account,
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

    println!("✅ Send Library Set Successfully!");
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
