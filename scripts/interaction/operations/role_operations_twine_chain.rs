use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{anyhow, Context, Result};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Signer, transaction::Transaction};
use twine_chain::core::{instruction as twine_chain_instruction, state::RoleType};

pub fn parse_role_type(s: &str) -> Result<RoleType> {
    match s.to_lowercase().as_str() {
        "messageappender" | "message_appender" => Ok(RoleType::MessageAppender),
        "twineoperationhandler" | "twine_operation_handler" => Ok(RoleType::TwineOperationHandler),
        _ => Err(anyhow!(
            "Invalid role type: '{}'. Valid options: 'MessageAppender', 'TwineOperationHandler'",
            s
        )),
    }
}

pub fn add_role_in_twine_chain(role_type: String, user_pubkey: Pubkey) -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let role_type = parse_role_type(&role_type).context("Failed to parse role type")?;

    let instructions = twine_chain_instruction::add_role_in_twine_chain(
        &account.pubkey(),
        &user_pubkey,
        role_type,
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

    println!("🎉 Role successfully added!");
    println!("📋 Transaction signature: {}", signature);
    println!("👤 User {} now has role: {:?}", user_pubkey, role_type);
    Ok(())
}
