use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn forced_native_token_withdrawal(
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
    let native_token_vault_data = next_account_info(account_info_iter)?;
    let native_token_vault = next_account_info(account_info_iter)?;
    let forced_withdrawal_messages_buffer = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let token_decimal_mappings = next_account_info(account_info_iter)?;
    let clock = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Perform necessary checks
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramError::InvalidArgument);
    }

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

    // Verify signature
    // let recovered_address = recover_address(withdraw_info.clone(), signature)?;
    // if recovered_address != from_twine_address {
    //     return Err(ProgramError::InvalidArgument);
    // }
    Ok(())
}
