use crate::core::error::ProgramCustomError;
use crate::core::state::{ForcedWithdrawMessageInfo, ForcedWithdrawMessagesBuffer};
use crate::utils::constants::{FORCED_WITHDRAWAL_BUFFER_PREFIX, ROLE_MANAGER_PREFIX};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn append_forced_withdrawal_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdraw_info: ForcedWithdrawMessageInfo,
) -> ProgramResult {
    // TODO: Add role manager so that only tokens_gateway program can call this function

    let account_info_iter = &mut accounts.iter();
    let forced_withdraw_message_buffer = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let initializer = next_account_info(account_info_iter)?;

    // Validate signer
    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and Validate PDA
    let (expected_withdraw_pda, _withdraw_bump_seed) =
        Pubkey::find_program_address(&[FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_withdraw_pda != *forced_withdraw_message_buffer.key {
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
        + (4 + withdraw_info.from_twine_address.len())
        + (4 + withdraw_info.to_l1_pubkey.len())
        + (4 + withdraw_info.l1_token.len())
        + (4 + withdraw_info.l2_token.len())
        + (4 + withdraw_info.amount.len());

    if total_len > ForcedWithdrawMessageInfo::LEN {
        return Err(ProgramCustomError::InvalidDataLength.into());
    }

    // Deserialize account data
    let mut withdrawals =
        ForcedWithdrawMessagesBuffer::try_from_slice(&forced_withdraw_message_buffer.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if withdraw message buffer is initialized
    if !withdrawals.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    // Update Withdrawals
    withdrawals.withdraw_messages.push(withdraw_info.clone());
    withdrawals.withdraw_nonce += 1;

    withdrawals
        .serialize(&mut &mut forced_withdraw_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // TODO: Emit Forced Withdraw Successful event

    Ok(())
}
