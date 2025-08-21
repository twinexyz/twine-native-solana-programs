use crate::utils::{get_default_keypair, get_or_create_ata, get_rpc_client};
use anyhow::{Context,Result};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction,
    utils::address_derivation::derive_spl_vault_authority, ID as tokens_gateway_ID,
};

pub fn spl_token_deposit(
    l1_token: Pubkey,
    l2_token: String,
    receiver_twine_address: String,
    user_token_account: Pubkey,
    amount: u64,
    data: String
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let spl_token_vault = get_or_create_ata(
        &rpc_client,
        &account,
        &derive_spl_vault_authority(&tokens_gateway_ID).0,
        &l1_token,
    );
    let instructions = tokens_gateway_instruction::spl_token_deposit(
        &account.pubkey(),
        &user_token_account,
        &l1_token,
        &spl_token_vault,
        receiver_twine_address,
        l1_token.to_string(),
        l2_token.clone(),
        amount,
        data
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
    println!("Spl Token Deposit successfull");

    Ok(())
}


