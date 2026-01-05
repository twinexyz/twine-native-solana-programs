use crate::utils::{get_default_keypair, get_or_create_ata, get_rpc_client};
use crate::tokens_gateway_client as tokens_gateway_instruction;
use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use tokens_gateway::{
    utils::address_derivation::derive_spl_vault_authority, ID as tokens_gateway_ID,
};

pub fn process_spl_l1_refund(
    l1_token: Pubkey,
    l1_receiver_address: Pubkey,
    public_value: String,
    proof: String,
) -> Result<()> {
    let public_values = hex::decode(public_value.trim_start_matches("0x"))?;
    let execution_proof = hex::decode(proof.trim_start_matches("0x"))?;
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let message_nonce = u64::from_be_bytes(public_values[41..49].try_into().unwrap());
    let spl_token_vault = get_or_create_ata(
        &rpc_client,
        &account,
        &derive_spl_vault_authority(&tokens_gateway_ID).0,
        &l1_token,
    );
    let instructions = tokens_gateway_instruction::process_spl_refund(
        &account.pubkey(),
        &l1_token,
        &spl_token_vault,
        l1_receiver_address,
        message_nonce,
        public_values,
        execution_proof,
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

    println!("✅ Spl token Refund successfull");
    println!("Transaction: {}", signature);
    Ok(())
}
