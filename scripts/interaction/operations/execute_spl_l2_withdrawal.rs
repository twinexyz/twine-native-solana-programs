use anyhow::{Context, Result};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
};

use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::id as spl_token_program_id;

use tokens_gateway::{
    core::instruction as tokens_gateway_instruction, id as tokens_gateway_id,
    utils::address_derivation::derive_spl_vault_authority,
};

use crate::utils::{get_default_keypair, get_rpc_client};

pub fn execute_spl_l2_withdrawal(
    spl_token_pubkey: Pubkey,
    l1_receiver_address: Pubkey,
    public_values: String,
    execution_proof: String,
) -> Result<()> {
    let public_values = hex::decode(public_values.trim_start_matches("0x"))?;
    let execution_proof = hex::decode(execution_proof.trim_start_matches("0x"))?;
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let spl_tokens_vault = get_or_create_ata(
        &rpc_client,
        &account,
        &derive_spl_vault_authority(&tokens_gateway_id()).0,
        &spl_token_pubkey,
    )?;

    let instructions = tokens_gateway_instruction::execute_l2_spl_withdrawal(
        &spl_token_pubkey,
        &spl_tokens_vault,
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

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ L2 Spl Withdrawal Executed!");
    println!("Transaction: {}", signature);
    Ok(())
}

pub fn get_or_create_ata(
    rpc_client: &RpcClient,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey> {
    let ata = get_associated_token_address(owner, mint);

    if rpc_client.get_account(&ata).is_err() {
        let create_ata_ix =
            create_associated_token_account(&payer.pubkey(), owner, mint, &spl_token_program_id());

        let blockhash = rpc_client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );

        rpc_client
            .send_and_confirm_transaction(&tx)
            .context("Failed to create associated token account")?;
    }

    Ok(ata)
}
