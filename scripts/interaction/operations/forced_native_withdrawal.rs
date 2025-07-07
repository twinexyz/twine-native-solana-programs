use crate::utils::{get_default_keypair, get_rpc_client,get_ethereum_signature};
use anyhow::{Context, Result};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction, core::state::SignMessageInfo,
};

pub fn forced_native_withdrawal(
    l1_token: String,
    l2_token: String,
    from_twine_address: String,
    privkey: String,
    amount: u64,
) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;

    let sign_info = SignMessageInfo {
        nonce: 1,
        chain_id: 900,
        amount: amount,
        from_twine_address: from_twine_address.to_string(),
        to_l1_pubkey:(account.pubkey()).to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };

    let signature = get_ethereum_signature(&sign_info, &privkey);

    let instructions = tokens_gateway_instruction::forced_native_token_withdrawal(
        &account.pubkey(),
        from_twine_address.clone(),
        (account.pubkey()).to_string(),
        l1_token.clone(),
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

    rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;
    print!("Forced Native Token Withdrawal");

    Ok(())
}
