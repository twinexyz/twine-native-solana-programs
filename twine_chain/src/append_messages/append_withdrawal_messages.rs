use borsh::{BorshDeserialize, BorshSerialize};
use serde_json;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            DetailedMessagesBuffer, MessageInfo, MessageTransactionEvent, MessagesBuffer, RoleType,
            TwineChainRoleManager,
        },
    },
    utils::{
        address_derivation::{derive_messages_buffer, derive_twine_chain_role_manager, verify_derived_address},
        constants::FORCED_WITHDRAW_MESSAGE_TYPE,
    },
};

pub fn append_forced_withdrawal_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdraw_info: MessageInfo,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        messages_buffer_acc,
        role_manager_acc,
        initializer_acc,
    )?;

    // Deserialize account data
    let mut withdrawals =
        DetailedMessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Deserialize account data
    let mut messages_buffer =
        MessagesBuffer::deserialize(&mut &detailed_messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if withdraw message buffer is initialized
    if !withdrawals.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }
    let current_transaction_hash = withdraw_info.calculate_message_hash();
    let previous_rolling_hash = messages_buffer.messages_rolling_hash;
    // Update Withdrawals
    withdrawals.messages.push(current_transaction_hash);

    withdrawals.message_nonce += 1;

    withdrawals
        .serialize(&mut &mut detailed_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    messages_buffer.update_rolling_hash(&current_transaction_hash);
    messages_buffer
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let event = MessageTransactionEvent {
        event: "MessageTransaction".to_string(),
        nonce: withdraw_info.nonce,
        l1_pubkey: withdraw_info.l1_pubkey,
        twine_address: withdraw_info.twine_address,
        l1_token: withdraw_info.l1_token,
        l2_token: withdraw_info.l2_token,
        chain_id: withdraw_info.chain_id,
        amount: withdraw_info.amount,
        data: withdraw_info.data,
        message_type: FORCED_WITHDRAW_MESSAGE_TYPE.to_string(),
        slot_number: withdraw_info.slot_number,
        previous_rolling_hash: previous_rolling_hash,
    };

    let serialized_event =
        serde_json::to_string(&event).map_err(|_| ProgramCustomError::FailedToSerializeEvent)?;
    msg!("{}", serialized_event);

    Ok(())
}
fn validate_accounts(
    program_id: &Pubkey,
    messages_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_messages_pda, _) = derive_messages_buffer(program_id);
    verify_derived_address(expected_messages_pda, messages_buffer_acc)?;

    let (expected_role_manager_pda, _) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Checks if signer has required role(MessageAppender)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(initializer_acc.key, RoleType::MessageAppender) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(())
}
