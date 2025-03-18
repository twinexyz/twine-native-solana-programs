use crate::core::error::ProgramCustomError;
use crate::core::state::{DepositMessageInfo, DepositMessagesBuffer};
use crate::utils::constants::{DEPOSIT_BUFFER_PREFIX, ROLE_MANAGER_PREFIX};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn append_deposit_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_info: DepositMessageInfo,
) -> ProgramResult {
    // TODO: Add role manager so that only tokens_gateway program can call this function

    let account_info_iter = &mut accounts.iter();
    let deposit_message_buffer = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let initializer = next_account_info(account_info_iter)?;

    // Validate signer
    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and Validate PDA
    let (expected_deposit_pda, _deposit_bump_seed) =
        Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_deposit_pda != *deposit_message_buffer.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate data length
    let total_len = 8
        + 8
        + 8
        + (4 + deposit_info.from_l1_pubkey.len())
        + (4 + deposit_info.to_twine_address.len())
        + (4 + deposit_info.l1_token.len())
        + (4 + deposit_info.l2_token.len())
        + (4 + deposit_info.amount.len());

    if total_len > DepositMessageInfo::LEN {
        return Err(ProgramCustomError::InvalidDataLength.into());
    }

    // Deserialize account data
    let mut deposits = DepositMessagesBuffer::try_from_slice(&deposit_message_buffer.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !deposits.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    // Update Deposits
    deposits.deposit_messages.push(deposit_info.clone());
    deposits.deposit_nonce += 1;

    deposits
        .serialize(&mut &mut deposit_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // TODO: Emit Deposit Successful event

    Ok(())
}
