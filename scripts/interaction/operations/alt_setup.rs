use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use solana_client::rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig};
use solana_sdk::{
    address_lookup_table::{instruction as alt_instruction, program as alt_program, state::AddressLookupTable},
    clock::Slot,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    signature::Signer,
    system_program,
    transaction::Transaction,
};
use solana_transaction_status::UiTransactionEncoding;
use std::{str::FromStr, thread::sleep, time::Duration};
// Import your program IDs and PDA derivation functions:
use borsh::BorshDeserialize;
use tokens_gateway::{utils::address_derivation::*, ID as tokens_gateway_id};
use twine_chain::utils::constants::MESSAGE_NONCE_GAP;
use twine_chain::{
    core::state::{LayerZeroInfo, TwineChainStorage},
    utils::address_derivation::*,
    ID as twine_chain_program_id,
};
use oapp::{
    core::state::{DVN_CONFIG_SEED, EXECUTOR_CONFIG_SEED},
    utils::address_derivation::*,
    ID as oapp_program_id,
};

pub fn create_and_extend_alt() -> Result<Pubkey> {
    // 1. Setup RPC client and payer (authority) keypair
    let payer = get_default_keypair();
    let rpc_client = get_rpc_client();

    // 2. Get a recent slot for ALT derivation (use a recent confirmed slot)
    let commitment = CommitmentConfig::confirmed();
    let tip: Slot = rpc_client
        .get_slot_with_commitment(commitment)
        .context("Failed to fetch current slot")?;
    let blocks = rpc_client
        .get_blocks(tip.saturating_sub(200), Some(tip))
        .context("Failed to fetch recent blocks")?;
    let recent_slot = *blocks
        .last()
        .context("No recent blocks returned; is your local validator producing blocks?")?;
    
    // 3. Create the Address Lookup Table (ALT) instruction and derive its address
    let (create_alt_ix, lookup_table_address) = alt_instruction::create_lookup_table(
        payer.pubkey(), // authority (also the payer here)
        payer.pubkey(), // payer (funds the account creation)
        recent_slot,
    );
    println!("🔑 Derived new Address Lookup Table address: {}", lookup_table_address);
    let (blockhash, _) = rpc_client
        .get_latest_blockhash_with_commitment(commitment)
        .context("Failed to get recent blockhash")?;
    let create_tx = Transaction::new_signed_with_payer(
        &[create_alt_ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    // Simulate the transaction to ensure it will succeed
    let sim = rpc_client.simulate_transaction_with_config(
        &create_tx,
        RpcSimulateTransactionConfig {
            sig_verify: false,
            commitment: Some(commitment), // simulate at confirmed level
            encoding: Some(UiTransactionEncoding::Base64),
            replace_recent_blockhash: false,
            ..Default::default()
        },
    )?;
    if let Some(err) = sim.value.err {
        println!("SIMULATION ERROR: {err:?}");
        if let Some(logs) = sim.value.logs {
            for l in logs {
                println!("{l}");
            }
        }
        anyhow::bail!("Create ALT simulation failed");
    }

    // 4. Send the create instruction in a transaction and WAIT for confirmation
    let sig: Signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &create_tx,
            CommitmentConfig::finalized(), // wait until transaction is finalized on chain
            RpcSendTransactionConfig {
                skip_preflight: true,                     // we already simulated, skip redundant preflight
                preflight_commitment: Some(commitment.commitment),
                ..Default::default()
            },
        )
        .context("Failed to send and confirm ALT creation transaction")?;
    println!("🧾 Create ALT tx signature: {}", sig);

    // Ensure the ALT account now exists on-chain
    let alt_account = rpc_client
        .get_account(&lookup_table_address)
        .context("ALT account not found after creation")?;
    if alt_account.owner != alt_program::id() {
        anyhow::bail!(
            "ALT owner mismatch. Expected {}, got {}",
            alt_program::id(),
            alt_account.owner
        );
    }
    println!("✅ ALT account created on-chain at {}", lookup_table_address);

    // 5. Prepare all addresses to add to the lookup table
    // Fetch TwineChainStorage and LayerZeroInfo data for dynamic PDA derivations
    let twine_chain_storage_key = derive_twine_chain_storage(&twine_chain_program_id).0;
    let twine_chain_storage_acc = rpc_client
        .get_account(&twine_chain_storage_key)
        .context("Failed to fetch TwineChainStorage account")?;
    let storage_data = TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data[..])
        .context("Failed to deserialize TwineChainStorage")?;
    let start_nonce = storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;
    let layer_zero_info_key = derive_layer_zero_info(&twine_chain_program_id).0;
    let layer_zero_info_acc = rpc_client
        .get_account(&layer_zero_info_key)
        .context("Failed to fetch LayerZeroInfo account")?;
    let lz_info = LayerZeroInfo::deserialize(&mut &layer_zero_info_acc.data[..])
        .context("Failed to deserialize LayerZeroInfo")?;
    // Values for deriving several PDAs below
    let dst_eid = lz_info.dst_eid;
    let receiver = lz_info.receiver;

    // Gather all the relevant program IDs and PDA addresses involved in the deposit process
    let store_account = derive_store_pda(&oapp_program_id).0;
    let addresses: Vec<Pubkey> = vec![
        // Core accounts from tokens_gateway and twine_chain programs:
        derive_native_token_vault(&tokens_gateway_id).0,
        derive_native_token_vault_data(&tokens_gateway_id).0,
        derive_messages_buffer(&twine_chain_program_id).0,
        derive_detailed_messages_buffer(&twine_chain_program_id).0,
        derive_token_decimal_mappings(&tokens_gateway_id).0,
        derive_twine_chain_role_manager(&twine_chain_program_id).0,
        derive_twine_chain_storage(&twine_chain_program_id).0,
        derive_messages_replicator(&twine_chain_program_id, start_nonce, end_nonce).0,
        // System program and core program IDs:
        system_program::id(),
        twine_chain_program_id,
        derive_layer_zero_info(&twine_chain_program_id).0,
        // LayerZero Endpoint/ULN related PDAs and program IDs:
        store_account,
        get_send_library_program(), // LayerZero send library program ID
        derive_send_library_config(&store_account, &dst_eid).0,
        derive_default_send_library_config(&dst_eid).0,
        derive_send_library_info().0,
        derive_endpoint_settings().0,
        derive_nonce(&store_account, &dst_eid, &receiver).0,
        derive_endpoint_event_authority().0,
        get_endpoint_id(), // LayerZero Endpoint program ID
        // LayerZero ULN (Universal LayerZero Network) related PDAs:
        derive_uln().0,
        derive_send_config(&dst_eid, &store_account).0,
        derive_default_send_config(&dst_eid).0,
        derive_library_event_authority().0,
        // Executor and DVN program accounts (given as constants in your code):
        Pubkey::from_str("2DvDmd8SGrsuSdSrG15GgM4Ay3tNT49oE4cPEJRmkiXc").unwrap(), // executor_program ID
        Pubkey::find_program_address(
            &[EXECUTOR_CONFIG_SEED],
            &Pubkey::from_str("2DvDmd8SGrsuSdSrG15GgM4Ay3tNT49oE4cPEJRmkiXc").unwrap(),
        )
        .0, // executor config PDA
        Pubkey::from_str("NativeLoader1111111111111111111111111111111").unwrap(), // native loader program (e.g. for price feeds)
        Pubkey::from_str("A8BmGrQ7vuNNy5URzvfM5nr8RvWodAheMxjMcBQULiUE").unwrap(), // dvn_program ID
        Pubkey::find_program_address(
            &[DVN_CONFIG_SEED],
            &Pubkey::from_str("A8BmGrQ7vuNNy5URzvfM5nr8RvWodAheMxjMcBQULiUE").unwrap(),
        )
        .0, // dvn config PDA
    ];
    println!("🔎 Total addresses to add to ALT: {}", addresses.len());

    // 6. Extend the ALT with addresses in manageable chunks
    const MAX_ADDRS_PER_TX: usize = 20; // ~20 addresses per extend transaction (avoid 1232-byte tx limit)
    let total_chunks = (addresses.len() + MAX_ADDRS_PER_TX - 1) / MAX_ADDRS_PER_TX;
    let mut tx_count = 0;
    for chunk in addresses.chunks(MAX_ADDRS_PER_TX) {
        tx_count += 1;
        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table_address,
            payer.pubkey(),        // authority (must sign as ALT authority)
            Some(payer.pubkey()),  // payer covers reallocation rent (if needed)
            chunk.to_vec(),        // new addresses to add in this chunk
        );
        let extend_tx = Transaction::new_signed_with_payer(
            &[extend_ix],
            Some(&payer.pubkey()),
            &[&payer],
            // Fetch a fresh recent blockhash for each transaction
            rpc_client.get_latest_blockhash().context("Failed to get recent blockhash for extend")?,
        );
        rpc_client
            .send_and_confirm_transaction(&extend_tx)
            .with_context(|| {
                format!(
                    "Failed to extend ALT (tx {} of {}) with {} addresses",
                    tx_count,
                    total_chunks,
                    chunk.len()
                )
            })?;
        println!(
            "✅ Extended ALT with {} addresses (transaction {} of {})",
            chunk.len(),
            tx_count,
            total_chunks
        );
    }
    println!("🎉 ALT extension complete. Lookup table now holds all required addresses.");

    // 7. (Optional) Fetch and display the stored addresses from the ALT account
    let final_alt_account = rpc_client
        .get_account(&lookup_table_address)
        .context("Failed to fetch the final ALT account data")?;
    let alt_state = AddressLookupTable::deserialize(&mut &final_alt_account.data[..])
        .context("Failed to deserialize ALT account state")?;
    let stored_addresses: Vec<Pubkey> = alt_state.addresses.to_vec();
    println!(
        "🔐 ALT {} contains {} addresses:",
        lookup_table_address,
        stored_addresses.len()
    );
    for (i, addr) in stored_addresses.iter().enumerate() {
        println!("  [{}] {}", i, addr);
    }

    Ok(lookup_table_address)
}
