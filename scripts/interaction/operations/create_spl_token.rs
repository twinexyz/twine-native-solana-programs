#![allow(clippy::arithmetic_side_effects)]
use crate::utils::{create_spl_and_mint, get_default_keypair, get_rpc_client};

pub fn create_spl_token() {
    let account = get_default_keypair();
    let rpc_client = get_rpc_client();

    let (spl_token_pubkey, user_token_account) = create_spl_and_mint(&rpc_client, &account, 9, 100);
    println!("Created Spl Token {}", spl_token_pubkey);
    println!("User token account {}", user_token_account);
}
