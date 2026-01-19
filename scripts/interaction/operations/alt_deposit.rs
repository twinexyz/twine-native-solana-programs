// alt_transaction.rs
use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use solana_sdk::{
    address_lookup_table::state::AddressLookupTable,
    message::{v0::Message as V0Message, VersionedMessage},
    pubkey::Pubkey,
    signature::Signer,
    transaction::{Transaction, VersionedTransaction},
};
// use solana_address_lookup_table_program::state::AddressLookupTable;
use crate::utils::{get_default_keypair, get_rpc_client};
use solana_message::AddressLookupTableAccount;
use std::str::FromStr;
use tokens_gateway::core::{instruction as tokens_gateway_instruction, state::LzMessageParams};
use twine_chain::{
    core::state::{LayerZeroInfo, TwineChainStorage},
    id as twine_chain_id,
    utils::{
        address_derivation::{derive_layer_zero_info, derive_twine_chain_storage},
        constants::MESSAGE_NONCE_GAP,
    },
};

const ALT_LOOKUP_TABLE_ADDRESS: &str = "3xHzLpgCJbQAnrGEZT2xAsx7JWXRTTcvxmEg7sX6aNLt"; // Replace with your ALT address

pub fn native_token_deposit_using_lz_alt(
    l1_token: String,
    l2_token: String,
    receiver_twine_address: String,
    amount: u64,
    data: String,
) -> Result<()> {
    let rpc_client = get_rpc_client();
    let payer = get_default_keypair();

    // 1. Fetch the lookup table account from chain
    let alt_pubkey =
        Pubkey::from_str(ALT_LOOKUP_TABLE_ADDRESS).context("Invalid ALT address string")?;
    let alt_account_data = rpc_client
        .get_account(&alt_pubkey)
        .context("Failed to fetch ALT account data")?;
    let alt_table = AddressLookupTable::deserialize(&alt_account_data.data[..])
        .context("Failed to deserialize ALT account")?;
    let alt_account = AddressLookupTableAccount {
        key: alt_pubkey,
        addresses: alt_table.addresses.to_vec(),
    };

    // 2. Prepare the LZ native deposit instruction (using ALT addresses)
    // Compute current start_nonce and end_nonce from TwineChainStorage
    let twine_chain_program_id = twine_chain_id();
    let storage_pda = derive_twine_chain_storage(&twine_chain_program_id).0;
    let storage_account = rpc_client
        .get_account(&storage_pda)
        .context("Failed to fetch TwineChainStorage PDA")?;
    let storage_data = TwineChainStorage::deserialize(&mut storage_account.data.as_ref())
        .context("Failed to deserialize TwineChainStorage")?;
    let start_nonce = storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;
    // Fetch LayerZero endpoint info (dst_eid and receiver)
    let lz_info_pda = derive_layer_zero_info(&twine_chain_program_id).0;
    let lz_info_account = rpc_client
        .get_account(&lz_info_pda)
        .context("Failed to fetch LayerZeroInfo PDA")?;
    let lz_info = LayerZeroInfo::deserialize(&mut &lz_info_account.data[..])
        .context("Failed to deserialize LayerZeroInfo")?;
    let params = LzMessageParams {
        dst_eid: lz_info.dst_eid,
        receiver: lz_info.receiver,
    };
    // DVN and Executor program accounts (same as earlier)
    let dvn_program = Pubkey::from_str("A8BmGrQ7vuNNy5URzvfM5nr8RvWodAheMxjMcBQULiUE").unwrap();
    let executor_program =
        Pubkey::from_str("2DvDmd8SGrsuSdSrG15GgM4Ay3tNT49oE4cPEJRmkiXc").unwrap();

    // Build the deposit instruction(s)
    let instructions = tokens_gateway_instruction::lz_native_token_deposit(
        &payer.pubkey(),
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        start_nonce,
        end_nonce,
        hex::decode(data).unwrap(),
        &dvn_program,
        &executor_program,
        params,
    );
    // (The above returns a Vec<Instruction>; in this case it's one instruction for the deposit.)

    // 3. Create a Versioned message that uses the ALT for additional addresses
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let v0_message = V0Message::try_compile(
        &payer.pubkey(), // fee payer
        &instructions,   // the instructions for this transaction
        &[alt_account],  // reference to our loaded lookup table account
        recent_blockhash,
    )
    .context("Failed to compile v0 message")?;
    let versioned_message = VersionedMessage::V0(v0_message);

    // 4. Convert to VersionedTransaction, sign it, and send
    let versioned_tx = VersionedTransaction::try_new(versioned_message, &[&payer])
        .context("Failed to create VersionedTransaction")?;
    let signature = rpc_client
        .send_and_confirm_transaction(&versioned_tx)
        .context("Failed to send and confirm versioned transaction")?;

    println!("✅ Lz Native token deposit successful via ALT!");
    println!("Transaction Signature: {}", signature);
    Ok(())
}
