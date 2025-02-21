use crate::core::error::ProgramCustomError;
use crate::core::state::SplTokensVaultData;
use crate::state::{SplTokensVault, TokenDecimalMappings};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};
use spl_token::instruction as token_instruction;
// use crate::utils::ethereum::is_valid_ethereum_address;

pub fn spl_tokens_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    twine_receiver: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data = next_account_info(account_info_iter)?;
    let spl_tokens_vault = next_account_info(account_info_iter)?;
    let vault_authority = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings = next_account_info(account_info_iter)?;
    let deposit_messages_buffer = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    if amount == 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if l1_token != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    let user_balance = spl_token::state::Account::unpack(&user_token_account.data.borrow())?.amount;
    if user_balance < amount {
        return Err(TokensGatewayError::InsufficientFundsForTransfer.into());
    }
    // Get the decimal mapping
    let token_decimal_mappings =
        TokenDecimalMappings::unpack(&token_decimal_mappings.data.borrow())?;
    let decimal_mapping = token_decimal_mappings
        .get_mapping(&l1_token)
        .ok_or(TokensGatewayError::TokenMappingNotFound)?;

    //todo: token decimal mapping checks and conversions
    let transfer_instruction = token_instruction::transfer(
        token_program.key,
        user_token_account.key,
        spl_tokens_vault.key,
        user.key,
        &[],
        amount,
    )?;
    invoke(
        &transfer_instruction,
        &[
            user_token_account.clone(),
            spl_tokens_vault.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;
    
    let mut spl_tokens_vault_data = SplTokensVault::unpack(&spl_tokens_vault_data.data.borrow())?;
    spl_tokens_vault_data.update_deposit(*mint.key, amount)?;
    SplTokensVault::pack(
        spl_tokens_vault_data,
        &mut spl_tokens_vault_data.data.borrow_mut(),
    )?;
    // todo:CPI for appending the mesage
    msg!("SPL token deposit successful");
    Ok(())
}
