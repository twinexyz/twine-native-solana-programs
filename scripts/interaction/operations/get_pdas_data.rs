
use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use twine_chain::core::state::{MessagesBuffer, TwineChainStorage};
use twine_chain::{
    id as twine_chain_program_id, utils::address_derivation::{derive_twine_chain_storage,derive_messages_buffer},
};
use crate::utils::get_rpc_client;

pub fn get_messages_buffer_data() -> Result<()> {
    let rpc_client = get_rpc_client();

    let messages_buffer_account = rpc_client
        .get_account(&derive_messages_buffer(&twine_chain_program_id()).0)
        .context("Failed to fetch PDA account")?;

    let messages_buffer_data =
        MessagesBuffer::deserialize(&mut &messages_buffer_account.data[..])
            .context("Failed to deserialize Deposit Buffer")?;

    println!("Message Buffer Data: {:?}", messages_buffer_data);

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