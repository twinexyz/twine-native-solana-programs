use crate::utils::get_rpc_client;
use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use tokens_gateway::{
    core::state::{ExecutedPayoutsBuffer, TokensGatewayRoleManager,TokenDecimalMappings},
    id as tokens_gateway_program_id,
    utils::address_derivation::{derive_executed_payouts_buffer, derive_gateway_role_manager,derive_token_decimal_mappings},
};
use twine_chain::core::state::{
    MessagesBuffer, MessagesReplicator, DetailedMessagesBuffer,TwineChainRoleManager, TwineChainStorage,
};
use twine_chain::{
    id as twine_chain_program_id,
    utils::address_derivation::{
        derive_messages_buffer,derive_detailed_messages_buffer, derive_messages_replicator, derive_twine_chain_role_manager,
        derive_twine_chain_storage,
    },
};

pub fn get_messages_buffer_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let messages_buffer_account = rpc_client
        .get_account(&derive_messages_buffer(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let messages_buffer_data = MessagesBuffer::deserialize(&mut &messages_buffer_account.data[..])
        .context("Failed to deserialize Message Buffer")?;

    println!("Message Buffer Data: {:?}", messages_buffer_data);

    Ok(())
}

pub fn get_detailed_messages_buffer_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let detailed_messages_buffer_account = rpc_client
        .get_account(&derive_detailed_messages_buffer(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let messages_buffer_data = DetailedMessagesBuffer::deserialize(&mut &detailed_messages_buffer_account.data[..])
        .context("Failed to deserialize Detailed Message Buffer")?;

    println!("Detailed Message Buffer Data: {:?}", messages_buffer_data);

    Ok(())
}
pub fn get_payouts_buffer_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let executed_payouts_buffer_account = rpc_client
        .get_account(&derive_executed_payouts_buffer(&tokens_gateway_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let executed_payouts_buffer_data =
        ExecutedPayoutsBuffer::deserialize(&mut &executed_payouts_buffer_account.data[..])
            .context("Failed to deserialize Executed Buffer")?;

    println!("Message Buffer Data: {:?}", executed_payouts_buffer_data);

    Ok(())
}

pub fn get_twine_chain_storage_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let twine_chain_storage_account = rpc_client
        .get_account(&derive_twine_chain_storage(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .context("Failed to deserialize Deposit Buffer")?;

    println!("Twine chain storage data: {:?}", twine_chain_storage_data);

    Ok(())
}

pub fn get_message_replicator_data(start_nonce: u64, end_nonce: u64) -> Result<()> {
    let rpc_client = get_rpc_client();

    let message_replicator_account = rpc_client
        .get_account(
            &derive_messages_replicator(&twine_chain_program_id(), start_nonce, end_nonce).0,
        )
        .context("Failed to fetch PDA account")?;

    let message_replicator_data: MessagesReplicator =
        MessagesReplicator::deserialize(&mut &message_replicator_account.data[..])
            .expect("Failed to deserialize Message Replicator Data");

    println!("Message Replicator Data: {:?}", message_replicator_data);

    Ok(())
}

pub fn get_twine_chain_role_manager_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let twine_role_manager_account = rpc_client
        .get_account(&derive_twine_chain_role_manager(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let twine_role_manager_data: TwineChainRoleManager =
        TwineChainRoleManager::deserialize(&mut &twine_role_manager_account.data[..])
            .expect("Failed to deserialize Twine Chain Rolemanager Data");

    println!(
        "Twine Chain RoleManager Data: {:?}",
        twine_role_manager_data
    );

    Ok(())
}

pub fn get_tokens_gateway_role_manager_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let tokens_gateway_role_manager_account = rpc_client
        .get_account(&derive_gateway_role_manager(&tokens_gateway_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let tokens_gateway_role_manager_data: TokensGatewayRoleManager =
        TokensGatewayRoleManager::deserialize(&mut &tokens_gateway_role_manager_account.data[..])
            .expect("Failed to deserialize Twine Chain Rolemanager Data");

    println!(
        "Tokens Gateway RoleManager Data: {:?}",
        tokens_gateway_role_manager_data
    );

    Ok(())
}

pub fn get_tokens_mapping_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let tokens_mapping_account = rpc_client
        .get_account(&derive_token_decimal_mappings(&tokens_gateway_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let tokens_mapping_account_data: TokenDecimalMappings =
        TokenDecimalMappings::deserialize(&mut &tokens_mapping_account.data[..])
            .expect("Failed to deserialize Twine Chain Rolemanager Data");

    println!(
        "Tokens Mapping Data: {:?}",
        tokens_mapping_account_data
    );

    Ok(())
}
