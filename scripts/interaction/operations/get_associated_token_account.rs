use crate::utils::{get_asscoiated_token_account};
use solana_sdk::{pubkey::Pubkey};

pub fn get_associated_token_account(wallet_address: Pubkey, spl_token_pubkey: Pubkey) {
    let user_token_account = get_asscoiated_token_account(&wallet_address, &spl_token_pubkey);
    println!("The Associated token account : {}", user_token_account);
}
