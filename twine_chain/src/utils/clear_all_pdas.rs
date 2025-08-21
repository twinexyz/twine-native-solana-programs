use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::core::{
    state::{ MessagesBuffer},
};

pub fn clear_all_pdas(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;

    let mut messages_buffer =
        MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Resetting deposit message buffer
    messages_buffer.messages.clear();
    messages_buffer.message_nonce = 0;

    messages_buffer
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])?;

    Ok(())
}