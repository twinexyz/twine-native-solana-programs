use crate::utils::{get_default_keypair, get_rpc_client};
use anyhow::{Context, Result};
use borsh::BorshSerialize;
use oapp::{
    core::{
        instruction as oapp_instructions,
        state::{ExecutorConfig, SetConfigParams, UlnConfig},
    },
    utils::address_derivation::derive_store_pda,
};
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};
use std::str::FromStr;

use oapp::ID as oapp_id;

pub fn set_config() -> Result<()> {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();
    let blockhash = rpc_client.get_latest_blockhash()?;
    let endpoint_id = Pubkey::from_str("76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6").unwrap();
    let send_lib = Pubkey::from_str("7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH").unwrap();

    let dvn_account = Pubkey::from_str("7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH").unwrap();
    let store_account = derive_store_pda(&oapp_id).0;

    let uln_config = UlnConfig {
        confirmations: 1,
        required_dvn_count: 1,
        optional_dvn_count: 0,
        optional_dvn_threshold: 0,
        required_dvns: vec![dvn_account],
        optional_dvns: vec![],
    };

    let executor_config = ExecutorConfig {
        max_message_size: 100000,
        executor: dvn_account,
    };

    let send_uln_config = SetConfigParams {
        oapp: store_account,
        eid: 40161,
        config_type: 2,
        config: uln_config.try_to_vec().unwrap(),
    };

    let executor = SetConfigParams {
        oapp: store_account,
        eid: 40161,
        config_type: 1,
        config: executor_config.try_to_vec().unwrap(),
    };

    let mut instructions = vec![];
    instructions.extend(oapp_instructions::set_config(
        send_uln_config,
        &endpoint_id,
        &send_lib,
    ));
    instructions.extend(oapp_instructions::set_config(
        executor,
        &endpoint_id,
        &send_lib,
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&account.pubkey()),
        &[&account],
        blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send and confirm transaction")?;

    println!("✅ Send Library Initialization Successful!");
    println!("Transaction: {}", signature);

    Ok(())
}
