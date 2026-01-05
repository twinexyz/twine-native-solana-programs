use crate::utils::{get_default_keypair, get_ethereum_signature, get_rpc_client};
use crate::tokens_gateway_client as tokens_gateway_instruction;
use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, instruction::Instruction, signature::Signer,
    transaction::Transaction,
};
use tokens_gateway::core::state::SignMessageInfo;
use twine_chain::{
    core::state::{MessagesBuffer, TwineChainStorage},
    id as twine_chain_program_id,
    utils::{
        address_derivation::{derive_messages_buffer, derive_twine_chain_storage},
        constants::MESSAGE_NONCE_GAP,
    },
};

pub fn forced_native_withdrawal(
    l1_token: String,
    l2_token: String,
    from_twine_address: String,
    l1_receiver: String,
    privkey: String,
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
        amount,
        l1_pubkey: l1_receiver.clone(),
        twine_address: from_twine_address.clone(),
        l1_token: l1_token.clone(),
        l2_token: l2_token.clone(),
    };

    let signature = get_ethereum_signature(&sign_info, &privkey);

    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;

    let instructions = tokens_gateway_instruction::forced_native_token_withdrawal(
        &account.pubkey(),
        from_twine_address,
        l1_receiver,
        l1_token,
        l2_token,
        amount,
        start_nonce,
        end_nonce,
        signature,
    );
    let cu_limit_ix: Instruction = ComputeBudgetInstruction::set_compute_unit_limit(600_000);
    let cu_price_ix: Instruction = ComputeBudgetInstruction::set_compute_unit_price(5_000);
    let mut all_instructions = vec![cu_limit_ix, cu_price_ix];

    all_instructions.extend(instructions);

    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ Forced Native Token Withdrawal Done!");
    println!("Transaction: {}", signature);

    Ok(())
}
