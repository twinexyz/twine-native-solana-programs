use borsh::{BorshDeserialize, BorshSerialize};
use serde_json;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            DetailedMessagesBuffer, LayerZeroInfo, MessageInfo, MessageTransactionEvent,
            MessagesBuffer, RoleType, TwineChainRoleManager,
        },
    },
    utils::{
        address_derivation::{
            derive_layer_zero_info, derive_messages_buffer, derive_twine_chain_role_manager,
            verify_derived_address,
        },
        constants::DEPOSIT_MESSAGE_TYPE,
    },
};

use oapp::{core::instruction::OAppInstruction, ID as oapp_program_id};

pub fn append_lz_deposit_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_info: MessageInfo,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;
    let layer_zero_info_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        messages_buffer_acc,
        role_manager_acc,
        initializer_acc,
        layer_zero_info_acc,
    )?;

    // <------------------ Accounts required for OApp::sendMessage ------------------>
    let store_acc = next_account_info(account_info_iter)?;
    let send_library_program_acc = next_account_info(account_info_iter)?;
    let send_library_config_acc = next_account_info(account_info_iter)?;
    let default_send_library_config_acc = next_account_info(account_info_iter)?;
    let send_library_info_acc = next_account_info(account_info_iter)?;
    let endpoint_acc = next_account_info(account_info_iter)?;
    let nonce_acc = next_account_info(account_info_iter)?;
    let endpoint_event_authority_acc = next_account_info(account_info_iter)?;
    let endpoint_program_acc = next_account_info(account_info_iter)?;

    // Accounts required by library send
    let uln_acc = next_account_info(account_info_iter)?;
    let send_config_acc = next_account_info(account_info_iter)?;
    let default_send_config_acc = next_account_info(account_info_iter)?;
    // let payer_acc = next_account_info(account_info_iter)?;
    // let treasury_acc = next_account_info(account_info_iter)?;
    let system_program_acc = next_account_info(account_info_iter)?;
    let library_event_authority_acc = next_account_info(account_info_iter)?;
    // let library_program = next_account_info(account_info_iter)?;

    // Remaining accounts for dvn and executor
    let executor_program_acc = next_account_info(account_info_iter)?;
    let executor_config_acc = next_account_info(account_info_iter)?;
    let price_feed_executor_acc = next_account_info(account_info_iter)?;
    // let price_feed_config_executor_acc = next_account_info(account_info_iter)?;

    let dvn_program_acc = next_account_info(account_info_iter)?;
    let dvn_config_acc = next_account_info(account_info_iter)?;
    // let price_feed_dvn_acc = next_account_info(account_info_iter)?;
    // let price_feed_config_dvn_acc = next_account_info(account_info_iter)?;

    // Deserialize account data
    let mut deposits =
        DetailedMessagesBuffer::deserialize(&mut &detailed_messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut messages_buffer =
        MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !deposits.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    let current_transaction_hash = deposit_info.calculate_message_hash();
    let previous_rolling_hash = messages_buffer.messages_rolling_hash;
    // Update Deposits
    deposits.messages.push(current_transaction_hash);
    deposits.message_nonce += 1;
    deposits
        .serialize(&mut &mut detailed_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    messages_buffer.update_rolling_hash(&current_transaction_hash);
    messages_buffer
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let event = MessageTransactionEvent {
        event: "MessageTransaction".to_string(),
        nonce: deposit_info.nonce,
        slot_number: deposit_info.slot_number,
        l1_pubkey: deposit_info.l1_pubkey,
        twine_address: deposit_info.twine_address,
        l1_token: deposit_info.l1_token,
        l2_token: deposit_info.l2_token,
        chain_id: deposit_info.chain_id,
        amount: deposit_info.amount,
        data: deposit_info.data,
        message_type: DEPOSIT_MESSAGE_TYPE.to_string(),
        previous_rolling_hash: previous_rolling_hash,
    };

    // <------------------------ CPI to OApp::sendMessage ------------------------>
    let layer_zero_info_data =
        LayerZeroInfo::deserialize(&mut &layer_zero_info_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let payload = OAppInstruction::SendMessage {
        dst_eid: 40161,
        receiver: layer_zero_info_data.receiver,
        message: event.abi_encode_packed(),
        options: layer_zero_info_data.options,
        native_fee: layer_zero_info_data.native_fee,
        lz_token_fee: layer_zero_info_data.lz_token_fee,
    };

    let params_data = payload.try_to_vec()?;
    let mut lz_send_data = vec![];
    lz_send_data.extend_from_slice(&params_data);

    // --- Construct account metas for CPI ---
    let mut cpi_accounts = vec![];
    cpi_accounts.push(AccountMeta::new(*store_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        *send_library_program_acc.key,
        false,
    ));
    cpi_accounts.push(AccountMeta::new(*send_library_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(
        *default_send_library_config_acc.key,
        false,
    ));
    cpi_accounts.push(AccountMeta::new_readonly(*send_library_info_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*endpoint_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*nonce_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*endpoint_event_authority_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*endpoint_program_acc.key, false));

    cpi_accounts.push(AccountMeta::new(*uln_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*send_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*default_send_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*initializer_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*initializer_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*system_program_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*library_event_authority_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*send_library_program_acc.key, false));

    cpi_accounts.push(AccountMeta::new_readonly(*executor_program_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*executor_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        *price_feed_executor_acc.key,
        false,
    ));
    cpi_accounts.push(AccountMeta::new_readonly(
        *system_program_acc.key,
        false,
    ));

    cpi_accounts.push(AccountMeta::new_readonly(*dvn_program_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*dvn_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*price_feed_executor_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        *system_program_acc.key,
        false,
    ));

    let send_ix = Instruction {
        program_id: oapp_program_id,
        accounts: cpi_accounts,
        data: lz_send_data,
    };

    // Invoke the OApp::sendMessage instruction
    invoke(
        &send_ix,
        &[
            store_acc.clone(),
            send_library_program_acc.clone(),
            send_library_config_acc.clone(),
            default_send_library_config_acc.clone(),
            send_library_info_acc.clone(),
            endpoint_acc.clone(),
            nonce_acc.clone(),
            endpoint_event_authority_acc.clone(),
            endpoint_program_acc.clone(),
            uln_acc.clone(),
            send_config_acc.clone(),
            default_send_config_acc.clone(),
            initializer_acc.clone(),
            initializer_acc.clone(),
            system_program_acc.clone(),
            library_event_authority_acc.clone(),
            send_library_program_acc.clone(),
            executor_program_acc.clone(),
            executor_config_acc.clone(),
            price_feed_executor_acc.clone(),
            system_program_acc.clone(),
            dvn_program_acc.clone(),
            dvn_config_acc.clone(),
            price_feed_executor_acc.clone(),
            system_program_acc.clone(),
        ],
    )?;

    // Emit the event
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
    layer_zero_info_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_messages_pda, _) = derive_messages_buffer(program_id);
    verify_derived_address(expected_messages_pda, messages_buffer_acc)?;

    let (expected_role_manager_pda, _) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    let (expected_layer_zero_pda, _) = derive_layer_zero_info(program_id);
    verify_derived_address(expected_layer_zero_pda, layer_zero_info_acc)?;

    // Checks if signer has required role(MessageAppender)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(initializer_acc.key, RoleType::MessageAppender) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(())
}