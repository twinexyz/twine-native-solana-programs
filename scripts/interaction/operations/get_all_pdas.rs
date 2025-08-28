
use anyhow::{Result};
use tokens_gateway::{
    id as tokens_gateway_id,
    utils::address_derivation::{
        derive_executed_withdrawals_buffer, derive_gateway_role_manager, derive_native_token_vault,
        derive_native_token_vault_data, derive_spl_tokens_vault_data, derive_spl_vault_authority,
        derive_token_decimal_mappings,
    },
};
use twine_chain::{
    id as twine_chain_id,
    utils::address_derivation::{
        derive_messages_buffer, derive_execution_message_buffer,derive_role_manager, derive_twine_chain_storage,derive_commitment_pda
    },
};

pub fn get_all_pdas() -> Result<()> {
    let token_gateway_role_manager = derive_gateway_role_manager(&tokens_gateway_id()).0;
    let native_token_vault_pda = derive_native_token_vault(&tokens_gateway_id()).0;
    let native_token_vault_data_pda = derive_native_token_vault_data(&tokens_gateway_id()).0;
    let spl_tokens_vault_pda = derive_spl_vault_authority(&tokens_gateway_id()).0;
    let spl_tokens_vault_data_pda = derive_spl_tokens_vault_data(&tokens_gateway_id()).0;
    let token_decimal_mappings_pda = derive_token_decimal_mappings(&tokens_gateway_id()).0;
    let executed_withdrawals_buffer = derive_executed_withdrawals_buffer(&tokens_gateway_id()).0;
    let messages_buffer_pda = derive_messages_buffer(&twine_chain_id()).0;
    let twine_chain_gateway_role_manager = derive_role_manager(&twine_chain_id()).0;
    let twine_chain_storage = derive_twine_chain_storage(&twine_chain_id()).0;
    let execution_message_buffer = derive_execution_message_buffer(&twine_chain_id()).0;

    println!(
        "Tokens Gateway Role Manager: {:?}",
        token_gateway_role_manager
    );
    println!("Native Token Vault PDA: {:?}", native_token_vault_pda);
    println!(
        "Native Token Vault Data PDA: {:?}",
        native_token_vault_data_pda
    );
    println!("SPL Tokens Vault Authority PDA: {:?}", spl_tokens_vault_pda);
    println!("SPL Tokens Vault Data PDA: {:?}", spl_tokens_vault_data_pda);
    println!(
        "Token Decimal Mappings PDA: {:?}",
        token_decimal_mappings_pda
    );
    println!(
        "Executed Message Withdrawal PDA: {:?}",
        executed_withdrawals_buffer
    );
    println!(
        "Messasges Buffer PDA: {:?} ",
        messages_buffer_pda
    );
    println!(
        "Twine Chain Rolemanager PDA :  {:?} ",
        twine_chain_gateway_role_manager
    );
    println!("Twine chain storage: {:?} ", twine_chain_storage);
    println!("Execution Message Buffer: {:?}", execution_message_buffer);
    Ok(())
}

pub fn get_batch_pda(batch_number: u64,) -> Result<()> {
    let batch_pda = derive_commitment_pda(&twine_chain_id(),batch_number).0;
    println!("The PDA Id of the batch number {batch_number} is {:?}",batch_pda);
    Ok(())
}