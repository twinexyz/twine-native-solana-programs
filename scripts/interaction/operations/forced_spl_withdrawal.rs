use crate::utils::{get_default_keypair, get_ethereum_signature, get_rpc_client};
use anyhow::{Context, Ok, Result};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction, core::state::SignMessageInfo,
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

    let sign_info = SignMessageInfo {
        nonce: 2,
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

    rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;
    print!("Forced SPL Token Withdrawal DONE");

    Ok(())
}
