use crate::utils::{get_default_keypair, get_rpc_client};
use crate::tokens_gateway_client as tokens_gateway_instruction;
use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use solana_sdk::{signature::Signer, transaction::Transaction};
use twine_chain::{
    core::state::TwineChainStorage,
    id as twine_chain_program_id,
    utils::{address_derivation::derive_twine_chain_storage, constants::MESSAGE_NONCE_GAP},
};

pub fn native_token_deposit(
    l1_token: String,
    l2_token: String,
    receiver_twine_address: String,
    amount: u64,
    data: String,
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let twine_chain_storage_account = rpc_client
        .get_account(&derive_twine_chain_storage(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .context("Failed to deserialize TwineChainStorage")?;

    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;

    let instructions = tokens_gateway_instruction::native_token_deposit(
        &account.pubkey(),
        receiver_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        start_nonce,
        end_nonce,
        hex::decode(data.clone()).unwrap(),
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

    println!("✅ Native token deposit successful!");
    println!("Transaction: {}", signature);
    Ok(())
}
