use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::Sysvar,
};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{DetailedMessagesBuffer, MessageInfo, TransactionType, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_messages_buffer, derive_messages_replicator,
            derive_twine_chain_role_manager, derive_twine_chain_storage, verify_system_program,
        },
        constants::{MESSAGES_BUFFER_PREFIX, MESSAGE_NONCE_GAP},
    },
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{NativeTokenVaultData, TokenDecimalMappings},
    },
    utils::{
        address_derivation::{
            derive_native_token_vault, derive_native_token_vault_data,
            derive_token_decimal_mappings, verify_derived_address,
        },
        constants::{CHAIN_ID, DEPOSIT_TRANSACTION, NATIVE_TOKEN_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

/// Handles native token (SOL) deposits
pub fn native_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }

    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let twine_chain_role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    let (twine_chain_storage_data, start_nonce, end_nonce) = validate_accounts(
        user_account,
        native_token_vault_acc,
        native_token_vault_data_acc,
        messages_buffer_acc,
        detailed_messages_buffer_acc,
        token_decimal_mappings_acc,
        twine_chain_role_manager_acc,
        system_program,
        twine_chain_program,
        twine_chain_storage_acc,
        messages_replicator_acc,
        program_id,
    )?;

    // Transfer SOL from user to vault
    let transfer_ix =
        system_instruction::transfer(user_account.key, native_token_vault_acc.key, amount);
    invoke(
        &transfer_ix,
        &[
            user_account.clone(),
            native_token_vault_acc.clone(),
            system_program.clone(),
        ],
    )?;

    // Update vault data
    let mut vault_data =
        NativeTokenVaultData::deserialize(&mut &native_token_vault_data_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    vault_data
        .serialize(&mut &mut native_token_vault_data_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // Token decimal mapping
    let token_decimal_mappings_data =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let decimal_mapping = token_decimal_mappings_data
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    if l2_token != decimal_mapping.l2_token.to_string() {
        return Err(ProgramCustomError::TokenMappingNotFound.into());
    }

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    // Prepare deposit info
    let deposit_message_buffer =
        DetailedMessagesBuffer::deserialize(&mut &detailed_messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let deposit_message_nonce = deposit_message_buffer.message_nonce + 1;
    let clock = Clock::get()?;

    let deposit_info = MessageInfo {
        txn_type: TransactionType::Deposit,
        nonce: deposit_message_nonce,
        chain_id: CHAIN_ID,
        slot_number: clock.slot,
        l1_pubkey: user_account.key.to_string(),
        twine_address: receiver_twine_address,
        l1_token,
        l2_token,
        amount: l2_amount,
        data,
    };

    let (_, native_data_bump) = derive_native_token_vault_data(program_id);
    let seeds = &[
        NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
        &[native_data_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Check nonce gap
    if deposit_message_buffer.message_nonce
        >= twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP
    {
        let payload = TwineChainInstruction::CopyMessagesBuffer;
        let mut copy_instruction_data = vec![];
        copy_instruction_data.extend(payload.try_to_vec().unwrap());

        if !messages_replicator_acc.is_writable {
            msg!("BUG: messages_replicator_acc not writable at outer level");
            return Err(ProgramError::InvalidAccountData);
        }

        let copy_instruction_accounts = vec![
            AccountMeta::new(*detailed_messages_buffer_acc.key, false),
            AccountMeta::new(*twine_chain_storage_acc.key, false),
            AccountMeta::new(*messages_replicator_acc.key, false),
            AccountMeta::new(*user_account.key, true),
            AccountMeta::new_readonly(*system_program.key, false),
        ];

        let copy_instruction = Instruction {
            program_id: *twine_chain_program.key,
            accounts: copy_instruction_accounts,
            data: copy_instruction_data,
        };

        invoke(
            &copy_instruction,
            &[
                detailed_messages_buffer_acc.clone(),
                twine_chain_storage_acc.clone(),
                messages_replicator_acc.clone(),
                user_account.clone(),
                system_program.clone(),
            ],
        )?;
    }

    // Append deposit message
    let payload = TwineChainInstruction::AppendDepositMessage { deposit_info };
    let mut append_instruction_data = vec![];
    append_instruction_data.extend(payload.try_to_vec().unwrap());

    let append_instruction_accounts = vec![
        AccountMeta::new(*messages_buffer_acc.key, false),
        AccountMeta::new(*detailed_messages_buffer_acc.key, false),
        AccountMeta::new(*twine_chain_role_manager_acc.key, false),
        AccountMeta::new_readonly(*native_token_vault_data_acc.key, true),
    ];

    let append_instruction = Instruction {
        program_id: *twine_chain_program.key,
        accounts: append_instruction_accounts,
        data: append_instruction_data,
    };

    invoke_signed(
        &append_instruction,
        &[
            messages_buffer_acc.clone(),
            detailed_messages_buffer_acc.clone(),
            twine_chain_role_manager_acc.clone(),
            native_token_vault_data_acc.clone(),
        ],
        signer_seeds,
    )?;

    Ok(())
}

fn validate_accounts(
    user: &AccountInfo,
    native_token_vault_acc: &AccountInfo,
    native_token_vault_data_acc: &AccountInfo,
    messages_buffer_acc: &AccountInfo,
    detailed_messages_buffer_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    system_program: &AccountInfo,
    twine_chain_program: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    messages_replicator_acc: &AccountInfo,
    program_id: &Pubkey,
) -> Result<(TwineChainStorage, u64, u64), ProgramError> {
    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_native_token_vault, _) = derive_native_token_vault(program_id);
    verify_derived_address(expected_native_token_vault, native_token_vault_acc)?;

    let (expected_native_token_vault_data, _) = derive_native_token_vault_data(program_id);
    verify_derived_address(
        expected_native_token_vault_data,
        native_token_vault_data_acc,
    )?;

    let (expected_messages_buffer, _) = derive_messages_buffer(&twine_chain_program_id);
    verify_derived_address(expected_messages_buffer, messages_buffer_acc)?;

    let (expected_detailed_messages_buffer, _) =
        derive_detailed_messages_buffer(&twine_chain_program_id);
    verify_derived_address(
        expected_detailed_messages_buffer,
        detailed_messages_buffer_acc,
    )?;

    let (expecte_token_decimal_mapping, _) = derive_token_decimal_mappings(program_id);
    verify_derived_address(expecte_token_decimal_mapping, token_decimal_mappings_acc)?;

    let (expected_role_manager, _) = derive_twine_chain_role_manager(&twine_chain_program_id);
    verify_derived_address(expected_role_manager, role_manager_acc)?;

    let (expected_twine_chain_storage, _) = derive_twine_chain_storage(&twine_chain_program_id);
    verify_derived_address(expected_twine_chain_storage, twine_chain_storage_acc)?;

    verify_system_program(system_program)?;

    if twine_chain_program.key != &twine_chain_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;

    let (expected_messages_replicator_pda, _) =
        derive_messages_replicator(&twine_chain_program_id, start_nonce, end_nonce);
    verify_derived_address(expected_messages_replicator_pda, messages_replicator_acc)?;

    Ok((twine_chain_storage_data, start_nonce, end_nonce))
}
