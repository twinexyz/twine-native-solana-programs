use crate::utils::{get_default_keypair, get_ethereum_signature, get_rpc_client};
use anyhow::{Context, Ok, Result};
use borsh::BorshDeserialize;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction,msg};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction, core::state::SignMessageInfo,
};
use twine_chain::{
    core::state::MessagesBuffer,
    id as twine_chain_program_id,
    utils::address_derivation::derive_messages_buffer,
};

pub fn forced_spl_withdrawal(
    l1_token: Pubkey,
    l2_token: String,
    from_twine_address: String,
    privkey: String,
    user_token_account: Pubkey,
    amount: u64,
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
   let messages_buffer_account = rpc_client
        .get_account(&derive_messages_buffer(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;
    let messages_buffer_data = MessagesBuffer::deserialize(&mut &messages_buffer_account.data[..])
        .context("Failed to deserialize Message Buffer")?;
    msg!("nonce here {:?}", messages_buffer_data.message_nonce + 1);

    let sign_info = SignMessageInfo {
        nonce: messages_buffer_data.message_nonce + 1,
        chain_id: 900,
        amount: amount,
        from_twine_address: from_twine_address.to_string(),
        to_l1_pubkey: user_token_account.to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };

    let signature = get_ethereum_signature(&sign_info, &privkey);

    let instructions = tokens_gateway_instruction::forced_spl_token_withdrawal(
        &account.pubkey(),
        &user_token_account,
        &l1_token,
        from_twine_address,
        l1_token.to_string(),
        l2_token.clone(),
        amount,
        signature,
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

    println!("✅ Forced SPL Token Withdrawal DONE!");
    println!("Transaction: {}", signature);

    Ok(())
}
