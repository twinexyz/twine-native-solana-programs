use crate::core::error::ProgramCustomError;
use crate::core::state::{TokenDecimalMappings};
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
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
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let forced_withdrawal_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let clock = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Perform necessary checks
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramError::InvalidArgument);
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if !is_valid_ethereum_address(&from_twine_address)? {
        return Err(ProgramCustomError::InvalidAccount.into());
    }
    if !is_valid_ethereum_address(&to_l1_pubkey)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }
    let token_decimal_mapping =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;
    let decimal_mapping = token_decimal_mapping
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;
    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

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

    // let recovered_address = recover_address(withdraw_info.clone(), signature)?;

    // Verify signature
    // let recovered_address = recover_address(withdraw_info.clone(), signature)?;
    // if recovered_address != from_twine_address {
    //     return Err(ProgramError::InvalidArgument);
    // }
    Ok(())
}
