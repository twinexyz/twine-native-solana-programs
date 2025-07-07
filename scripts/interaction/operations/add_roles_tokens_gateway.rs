use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tokens_gateway::core::instruction as tokens_gateway_instruction;

pub fn update_token_mapping(l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,)-> Result<()> {
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

     rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;
    print!("Token Mapping Done");
    Ok(())
  
}
