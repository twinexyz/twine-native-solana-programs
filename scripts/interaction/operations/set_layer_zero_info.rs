use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};
use twine_chain::core::instruction as twine_chain_instructions;

pub fn set_layer_zero_info(dst_eid: u32, dst_oapp_address: String) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions =
        twine_chain_instructions::set_layer_zero_info(&account.pubkey(), dst_eid, dst_oapp_address);

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ LayerZero Info Updated Successfully!");
    println!("Transaction: {}", signature);

    Ok(())
}
