use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use std::str::FromStr;
use tokens_gateway::core::instruction as tokens_gateway_instruction;

pub fn execute_native_l2_withdrawal() -> Result<()> {
    let l1_receiver_address = Pubkey::from_str("BdhpXtonNKnVKpEK7iSzZvVU1gKSWtMjUaTuQZ4rvJkS")
        .context("Invalid L1 receiver address")?;
    let public_values = vec![
        0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 12, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 66, 100, 104, 112, 88, 116, 111, 110,
        78, 75, 110, 86, 75, 112, 69, 75, 55, 105, 83, 122, 90, 118, 86, 85, 49, 103, 75, 83, 87,
        116, 77, 106, 85, 97, 84, 117, 81, 90, 52, 114, 118, 74, 107, 83, 49, 49, 49, 49, 49, 49,
        49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49,
        49, 49, 49, 48, 120, 102, 51, 57, 70, 100, 54, 101, 53, 49, 97, 97, 100, 56, 56, 70, 54,
        70, 52, 99, 101, 54, 97, 66, 56, 56, 50, 55, 50, 55, 57, 99, 102, 102, 70, 98, 57, 50, 50,
        54, 54, 53, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
    ];
    let execution_proof = vec![];
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let instructions = tokens_gateway_instruction::execute_l2_native_withdrawal(
        l1_receiver_address,
        public_values,
        execution_proof,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to execute L2 Native withdrawal")?;
    print!("L2 Native Withdrawal Executed");
    Ok(())
}
