use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    program_error::ProgramError,
    program::{invoke, invoke_signed},
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use crate::core::error::ProgramCustomError;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
// use spl_token::instruction as token_instruction;

pub fn forced_spl_token_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let user = next_account_info(account_info_iter)?;
    let to_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data = next_account_info(account_info_iter)?;
    let spl_tokens_vault = next_account_info(account_info_iter)?;
    let vault_authority = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings = next_account_info(account_info_iter)?;
    let forced_withdrawal_messages_buffer = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let clock = next_account_info(account_info_iter)?;

    // Perform necessary checks
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if !is_valid_ethereum_address(&from_twine_address)? {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    // if Pubkey::from_str(&to_l1_pubkey).is_err() {
    //     return Err(ProgramError::InvalidArgument);
    // }
    // let withdraw_info = ForcedWithdrawMessageInfo {
    //     nonce: get_next_nonce(forced_withdrawal_messages_buffer)?,
    //     chain_id: 900,
    //     slot_number: clock.slot,
    //     from_twine_address: from_twine_address.clone(),
    //     to_l1_pubkey,
    //     l1_token,
    //     l2_token,
    //     amount: l2_amount.to_string(),
    // };
    Ok(())
}