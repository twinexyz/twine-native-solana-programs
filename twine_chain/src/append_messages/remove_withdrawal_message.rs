use crate::core::error::ProgramCustomError;
use crate::core::state::{ExecutionMessageBuffer, RoleType, TwineChainRoleManager};
use crate::utils::constants::{EXECUTION_BUFFER_PREFIX, ROLE_MANAGER_PREFIX};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn remove_withdrawal_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nonce: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let execution_message_buffer = next_account_info(account_iter)?;
    let role_manager = next_account_info(account_iter)?;
    let message_appender = next_account_info(account_iter)?;

    // Validate signer
    if !message_appender.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and Validate PDAs
    let (expected_execution_pda, _deposit_bump_seed) =
        Pubkey::find_program_address(&[EXECUTION_BUFFER_PREFIX.as_bytes()], program_id);
    if expected_execution_pda != *execution_message_buffer.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Check if initiator has MessageAppender Role
    let role_manager_data = TwineChainRoleManager::try_from_slice(&role_manager.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(message_appender.key, RoleType::MessageAppender) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    let mut execution_buffer_data =
        ExecutionMessageBuffer::try_from_slice(&execution_message_buffer.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !execution_buffer_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    let index = execution_buffer_data
        .withdrawals
        .iter()
        .position(|msg| msg.nonce == nonce)
        .ok_or(ProgramCustomError::NonceNotFound)?;

    // Remove the withdrawal message at the specified index
    execution_buffer_data.withdrawals.remove(index);

    execution_buffer_data
        .serialize(&mut &mut execution_message_buffer.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    Ok(())
}
