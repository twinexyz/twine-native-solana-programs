use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::json;
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
        state::{DepositMessageInfo, MessagesBuffer, RoleType, TwineChainRoleManager},
    },
    utils::{
        address_derivation::{derive_messages_buffer, derive_role_manager, verify_derived_address},
        constants::DEPOSIT_MESSAGE_TYPE,
    },
};

pub fn append_deposit_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_info: DepositMessageInfo,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        messages_buffer_acc,
        role_manager_acc,
        initializer_acc,
    )?;

    // Deserialize account data
    let mut deposits = MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !deposits.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    // Update Deposits
    deposits
        .messages
        .push(deposit_info.calculate_deposit_hash());
    deposits.message_nonce += 1;

    deposits
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let event = json!(
        {
            "event": "MessageTransaction",
            "nonce": deposit_info.nonce,
            "l1_pubkey": deposit_info.from_l1_pubkey,
            "twine_address": deposit_info.to_twine_address,
            "l1_token": deposit_info.l1_token,
            "l2_token": deposit_info.l2_token,
            "chain_id": deposit_info.chain_id,
            "amount": deposit_info.amount,
            "data": deposit_info.data,
            "message_type": DEPOSIT_MESSAGE_TYPE,
            "slot_number": deposit_info.slot_number

        }
    )
    .to_string();
    msg!(&event);
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

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
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