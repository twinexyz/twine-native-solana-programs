use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};
use twine_chain::{
    core::{instruction as twine_chain_instruction},

};

pub fn copy_messages_buffer(start_nonce: u64, end_nonce: u64) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions =
        twine_chain_instruction::copy_messages_buffer(&account.pubkey(), start_nonce, end_nonce);

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

   let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;
    print!("Message Buffer Replicated {:?}",signature);
    Ok(())
}
