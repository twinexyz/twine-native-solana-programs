use crate::utils::{get_default_keypair, get_rpc_client};
use crate::tokens_gateway_client as tokens_gateway_instruction;
use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

pub fn execute_native_l2_withdrawal(
    l1_receiver_address: Pubkey,
    public_value: String,
    proof: String,
) -> Result<()> {
    let public_values = hex::decode(public_value.trim_start_matches("0x"))?;
    let execution_proof = hex::decode(proof.trim_start_matches("0x"))?;
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let message_nonce =  u64::from_be_bytes(public_values[8..16].try_into().unwrap());
    let instructions = tokens_gateway_instruction::execute_l2_native_withdrawal(
        &account.pubkey(),
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

    println!("✅ L2 Native Withdrawal Executed!");
    println!("Transaction: {}", signature);
    Ok(())
}
