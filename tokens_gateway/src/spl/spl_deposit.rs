use crate::core::error::ProgramCustomError;
use crate::core::state::{SplTokensVaultData, TokenDecimalMappings};
use spl_token::solana_program::program_pack::Pack;
use spl_token::instruction as token_instruction;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use spl_token::state::Account as TokenAccount;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    system_instruction,
};
use solana_program::pubkey::Pubkey;
use spl_token::instruction;
use spl_token::{
    instruction::transfer_checked,
    state::{Account, Mint},
};

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
    if !is_valid_ethereum_address(&twine_receiver)? {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    let user_token_data = TokenAccount::unpack(&user_token_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    if user_token_data.amount < amount {
        msg!("User token account has insufficient funds");
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }
    let token_decimal_mappings =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings.data.borrow())?;
    let decimal_mapping = token_decimal_mappings
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

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
    let mut spl_tokens_vault_data =
    SplTokensVaultData::try_from_slice(&spl_tokens_vault_data.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    spl_tokens_vault_data.update_deposit(*mint.key, amount)?;
    // todo:CPI for appending the mesage
    msg!("SPL token deposit successful");
    Ok(())
}
