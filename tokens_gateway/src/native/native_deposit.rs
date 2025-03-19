use crate::core::error::ProgramCustomError;
use crate::core::state::NativeTokenVaultData;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
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
// Handles native token (SOL) deposits
pub fn native_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }
    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }
    let account_info_iter = &mut accounts.iter();

    // Account[0]: the depositor (user) - must be a signer
    let user_account = next_account_info(account_info_iter)?;
    let native_vault_account = next_account_info(account_info_iter)?;
    let native_vault_data_account = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if user_account.lamports() < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    if native_vault_data_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let transfer_ix =
        system_instruction::transfer(user_account.key, native_vault_account.key, amount);
    invoke(
        &transfer_ix,
        &[
            user_account.clone(),
            native_vault_account.clone(),
            system_program_info.clone(),
        ],
    )?;

    let mut vault_data =
        NativeTokenVaultData::try_from_slice(&native_vault_data_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    vault_data
        .serialize(&mut *native_vault_data_account.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!(
        "Native token deposit successful: {} lamports deposited",
        amount
    );
    Ok(())
}
