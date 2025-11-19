use crate::utils::{get_default_keypair, get_ethereum_signature, get_rpc_client};
use anyhow::{Context, Ok, Result};
use borsh::BorshDeserialize;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signature::Signer,
    transaction::Transaction,
};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction, core::state::SignMessageInfo,
};
use twine_chain::{
    core::state::{MessagesBuffer, TwineChainStorage},
    id as twine_chain_program_id,
    utils::{
        address_derivation::{derive_messages_buffer, derive_twine_chain_storage},
        constants::MESSAGE_NONCE_GAP,
    },
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
    let twine_chain_storage_account = rpc_client
        .get_account(&derive_twine_chain_storage(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .context("Failed to deserialize TwineChainStorage")?;

    let sign_info = SignMessageInfo {
        nonce: messages_buffer_data.message_nonce + 1,
        chain_id: 900,
        amount: amount,
        l1_pubkey: user_token_account.to_string(),
        twine_address: from_twine_address.to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };

    let signature = get_ethereum_signature(&sign_info, &privkey);

    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;
    let mut instructions = Vec::new();

    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(400_000);
    instructions.push(compute_budget_ix);

    let withdrawal_instructions = tokens_gateway_instruction::forced_spl_token_withdrawal(
        &account.pubkey(),
        &user_token_account,
        &l1_token,
        from_twine_address,
        l1_token.to_string(),
        l2_token.clone(),
        amount,
        start_nonce,
        end_nonce,
        signature,
    );

    instructions.extend(withdrawal_instructions);

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
