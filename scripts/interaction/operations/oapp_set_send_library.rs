use std::str::FromStr;

use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use oapp::{
    core::{instruction as oapp_instructions, state::SetSendLibraryParams},
    utils::address_derivation::derive_store_pda,
};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

use oapp::ID as oapp_id;

pub fn set_send_library() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let send_lib = Pubkey::from_str("2XgGZG4oP29U3w5h4nTk1V2LFHL23zKDPJjs3psGzLKQ").unwrap();

    let store_account = derive_store_pda(&oapp_id).0;
    let params = SetSendLibraryParams {
        sender: store_account,
        eid: 40161,
        new_lib: send_lib,
    };

    let instructions = oapp_instructions::set_send_library(params);

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
