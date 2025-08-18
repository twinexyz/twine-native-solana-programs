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
        state::{ForcedWithdrawMessageInfo, MessagesBuffer, RoleType, TwineChainRoleManager},
    },
    utils::{
        address_derivation::{derive_messages_buffer, derive_role_manager, verify_derived_address},
        constants::FORCED_WITHDRAW_MESSAGE_TYPE,
    },
};

pub fn append_forced_withdrawal_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdraw_info: ForcedWithdrawMessageInfo,
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

    // Validate data length
    // let total_len = 8
    //     + 8
    //     + 8
    //     + (4 + withdraw_info.from_twine_address.len())
    //     + (4 + withdraw_info.to_l1_pubkey.len())
    //     + (4 + withdraw_info.l1_token.len())
    //     + (4 + withdraw_info.l2_token.len())
    //     + (4 + withdraw_info.amount.len());

    // if total_len > ForcedWithdrawMessageInfo::LEN {
    //     return Err(ProgramCustomError::InvalidDataLength.into());
    // }

    // Deserialize account data
    let mut withdrawals = MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if withdraw message buffer is initialized
    if !withdrawals.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }
    // Update Withdrawals
    withdrawals
        .messages
        .push(withdraw_info.calculate_withdraw_hash());

    withdrawals.message_nonce += 1;

    withdrawals
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let event = json!(
        {
            "event": "MessageTransaction",
            "nonce": withdraw_info.nonce,
            "from_l1_pubkey": withdraw_info.from_twine_address,
            "to_twine_address": withdraw_info.to_l1_pubkey,
            "l1_token": withdraw_info.l1_token,
            "l2_token": withdraw_info.l2_token,
            "chain_id": withdraw_info.chain_id,
            "amount": withdraw_info.amount,
            "data": "",
            "message_type": FORCED_WITHDRAW_MESSAGE_TYPE,
            "slot_number": withdraw_info.slot_number

        }
    )
    .to_string();
    msg!(&event);
    Ok(())
}
// TransactionType.Withdraw,
//             messageIndex,
//             chainId,
//             uint64(block.number),
//             l1Token,
//             l2Token,
//             from,
//             to,
//             amount,
//             message
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

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_QUEUE_SIZE, MAX_ROLES};
//     use solana_program::{clock::Epoch, rent::Rent, system_program};
//     use std::str::FromStr;

//     fn create_test_account_info<'a>(
//         key: &'a Pubkey,
//         is_signer: bool,
//         is_writable: bool,
//         lamports: &'a mut u64,
//         data: &'a mut [u8],
//         owner: &'a mut Pubkey,
//     ) -> AccountInfo<'a> {
//         AccountInfo::new(
//             key,
//             is_signer,
//             is_writable,
//             lamports,
//             data,
//             owner,
//             false,
//             Epoch::default(),
//         )
//     }

//     #[test]
//     fn test_append_withdrawal_messages() -> Result<(), Box<dyn std::error::Error>> {
//         let program_id = Pubkey::new_unique();

//         // Get the required accounts
//         let (withdraw_message_buffer_key, _) = derive_forced_withdraw_message_buffer(&program_id);
//         let (role_manager_key, _) = derive_role_manager(&program_id);
//         let initializer_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
//         let system_program_id = system_program::id();

//         // Required space for each account
//         let withdraw_message_buffer_space: usize =
//             1 + 8 + 4 + (MAX_QUEUE_SIZE * ForcedWithdrawMessageInfo::LEN);
//         let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

//         // Setup Account Lamports
//         let rent = Rent::default();
//         let mut withdraw_message_buffer_lamports =
//             rent.minimum_balance(withdraw_message_buffer_space);
//         let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
//         let mut initializer_lamports = 1_000_000_000;

//         // Setup Account Data
//         let buffer = ForcedWithdrawMessagesBuffer {
//             is_initialized: true,
//             withdraw_nonce: 0,
//             withdraw_messages: vec![],
//         };

//         let mut withdraw_buffer_data = vec![0u8; withdraw_message_buffer_space];

//         let mut temp = vec![];
//         buffer.serialize(&mut temp)?;

//         withdraw_buffer_data[..temp.len()].copy_from_slice(&temp);

//         // Gives role TwineOperationHandler to InitialChainAdmin
//         let role_manager_dummy_data = TwineChainRoleManager {
//             is_initialized: true,
//             chain_admin: Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
//             twine_operator: Pubkey::default(),
//             token_gateway_program: Pubkey::default(),
//             roles: vec![(
//                 Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
//                 RoleType::MessageAppender,
//             )],
//         };
//         let mut role_manager_data = vec![];
//         role_manager_dummy_data.serialize(&mut role_manager_data)?;

//         let mut initializer_data = vec![];

//         // Setup owners
//         let mut withdraw_message_buffer_owner = program_id;
//         let mut role_manager_owner = program_id;
//         let mut initializer_owner = system_program_id;

//         // Create required account infos
//         let withdraw_message_buffer_account = create_test_account_info(
//             &withdraw_message_buffer_key,
//             false,
//             true,
//             &mut withdraw_message_buffer_lamports,
//             &mut withdraw_buffer_data,
//             &mut withdraw_message_buffer_owner,
//         );

//         let role_manager_account = create_test_account_info(
//             &role_manager_key,
//             false,
//             true,
//             &mut role_manager_lamports,
//             &mut role_manager_data,
//             &mut role_manager_owner,
//         );

//         let initializer_account = create_test_account_info(
//             &initializer_key,
//             true,
//             false,
//             &mut initializer_lamports,
//             &mut initializer_data,
//             &mut initializer_owner,
//         );

//         // Create accounts array in the correct order matching the function
//         let accounts = vec![
//             withdraw_message_buffer_account.clone(),
//             role_manager_account.clone(),
//             initializer_account.clone(),
//         ];

//         // call the set function
//         let dummy_msg = ForcedWithdrawMessageInfo {
//             nonce: 1,
//             chain_id: 100,
//             slot_number: 200,
//             to_l1_pubkey: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
//             from_twine_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
//             l1_token: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
//             l2_token: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
//             amount: "1000000000000000000".to_string(),
//         };

//         let result = append_forced_withdrawal_message(&program_id, &accounts, dummy_msg);
//         assert!(result.is_ok(), "Setter failed: {:?}", result.err());

//         // verify setter
//         let withdraw_buffer_data = ForcedWithdrawMessagesBuffer::deserialize(
//             &mut &withdraw_message_buffer_account.data.borrow()[..],
//         )?;

//         assert_eq!(
//             withdraw_buffer_data.withdraw_messages.len(),
//             1,
//             "Value should be set to value provided"
//         );
//         Ok(())
//     }
// }
