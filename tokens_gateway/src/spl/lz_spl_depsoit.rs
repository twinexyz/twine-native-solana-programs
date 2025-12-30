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
    sysvar::Sysvar,
};
use spl_token::{
    instruction as token_instruction, solana_program::program_pack::Pack,
    state::Account as TokenAccount,
};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{DetailedMessagesBuffer, MessageInfo, TransactionType, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_layer_zero_info, derive_messages_buffer,
            derive_messages_replicator, derive_twine_chain_role_manager,
            derive_twine_chain_storage, verify_system_program,
        },
        constants::MESSAGE_NONCE_GAP,
    },
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{SplTokensVaultData, TokenDecimalMappings},
    },
    utils::{
        address_derivation::{
            derive_native_token_vault_data, derive_spl_tokens_vault_data,
            derive_token_decimal_mappings, verify_derived_address,
        },
        constants::{CHAIN_ID, DEPOSIT_TRANSACTION, SPL_TOKENS_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn lz_spl_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let messages_replicator_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;
    let layer_zero_info_acc = next_account_info(account_info_iter)?;

    if l1_token != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidToken.into());
    }

    let (twine_chain_storage_data, start_nonce, end_nonce) = validate_accounts(
        user,
        user_token_account,
        spl_tokens_vault_data_acc,
        spl_tokens_vault_acc,
        mint,
        token_program,
        token_decimal_mappings_acc,
        messages_buffer_acc,
        detailed_messages_buffer_acc,
        twine_chain_role_manager_acc,
        twine_chain_program,
        twine_chain_storage_acc,
        messages_replicator_acc,
        system_program,
        layer_zero_info_acc,
        program_id,
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
    let payer_acc = next_account_info(account_info_iter)?;
    let treasury_acc = next_account_info(account_info_iter)?;
    let system_program_acc = next_account_info(account_info_iter)?;
    let library_event_authority_acc = next_account_info(account_info_iter)?;
    let library_program = next_account_info(account_info_iter)?;

    // Remaining accounts for dvn and executor
    let executor_program_acc = next_account_info(account_info_iter)?;
    let executor_config_acc = next_account_info(account_info_iter)?;
    let price_feed_executor_acc = next_account_info(account_info_iter)?;
    let price_feed_config_executor_acc = next_account_info(account_info_iter)?;

    let dvn_program_acc = next_account_info(account_info_iter)?;
    let dvn_config_acc = next_account_info(account_info_iter)?;
    let price_feed_dvn_acc = next_account_info(account_info_iter)?;
    let price_feed_config_dvn_acc = next_account_info(account_info_iter)?;

    let spl_data_seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes(), &[spl_data_bump]];
    let signer_seeds = &[&seeds[..]];

    let user_token_data = TokenAccount::unpack(&user_token_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if user_token_data.amount < amount {
        msg!("User token account has insufficient funds");
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }

    let token_decimal_mappings =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])?;

    let decimal_mapping = token_decimal_mappings
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

    let transfer_instruction = token_instruction::transfer(
        &spl_token::id(),
        &user_token_account.key,
        &spl_tokens_vault_acc.key,
        &user.key,
        &[],
        amount,
    )?;
    invoke(
        &transfer_instruction,
        &[
            user_token_account.clone(),
            spl_tokens_vault_acc.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;

    let mut spl_tokens_vault_data =
        SplTokensVaultData::deserialize(&mut &spl_tokens_vault_data_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    spl_tokens_vault_data.update_deposit(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut &mut spl_tokens_vault_data_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let deposit_message_buffer =
        DetailedMessagesBuffer::deserialize(&mut &detailed_messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = deposit_message_buffer.message_nonce + 1;

    let clock = Clock::get()?;

    let deposit_info = MessageInfo {
        txn_type: TransactionType::Deposit,
        nonce: u64_nonce,
        chain_id: CHAIN_ID,
        slot_number: clock.slot,
        l1_pubkey: user_token_account.key.to_string(),
        twine_address: receiver_twine_address,
        l1_token: l1_token,
        l2_token: l2_token,
        amount: l2_amount,
        data: data,
    };

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
            AccountMeta::new(*user.key, true),
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
                user.clone(),
                system_program.clone(),
            ],
        )?;
    }

    let payload = TwineChainInstruction::AppendLzDepositMessage { deposit_info };

    let mut append_instruction_data = vec![];
    append_instruction_data.extend(payload.try_to_vec().unwrap());

    let mut append_instruction_accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(*messages_buffer_acc.key, false),
        AccountMeta::new(*detailed_messages_buffer_acc.key, false),
        AccountMeta::new_readonly(*twine_chain_role_manager_acc.key, false),
        AccountMeta::new_readonly(*spl_tokens_vault_data_acc.key, true),
        AccountMeta::new(*layer_zero_info_acc.key, false),
    ];

    // LayerZero Accounts
    append_instruction_accounts.push(AccountMeta::new(*store_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(
        *send_library_program_acc.key,
        false,
    ));
    append_instruction_accounts.push(AccountMeta::new(*send_library_config_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(
        *default_send_library_config_acc.key,
        false,
    ));
    append_instruction_accounts.push(AccountMeta::new_readonly(*send_library_info_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*endpoint_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*nonce_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*endpoint_event_authority_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(*endpoint_program_acc.key, false));

    append_instruction_accounts.push(AccountMeta::new(*uln_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*send_config_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*default_send_config_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*payer_acc.key, true));
    append_instruction_accounts.push(AccountMeta::new(*treasury_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(*system_program_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*library_event_authority_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(*library_program.key, false));

    append_instruction_accounts.push(AccountMeta::new_readonly(*executor_program_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*executor_config_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(
        *price_feed_executor_acc.key,
        false,
    ));
    append_instruction_accounts.push(AccountMeta::new_readonly(
        *price_feed_config_executor_acc.key,
        false,
    ));

    append_instruction_accounts.push(AccountMeta::new_readonly(*dvn_program_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new(*dvn_config_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(*price_feed_dvn_acc.key, false));
    append_instruction_accounts.push(AccountMeta::new_readonly(
        *price_feed_config_dvn_acc.key,
        false,
    ));

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
            spl_tokens_vault_data_acc.clone(),
            layer_zero_info_acc.clone(),
            // LayerZero Accounts
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
            payer_acc.clone(),
            treasury_acc.clone(),
            system_program_acc.clone(),
            library_event_authority_acc.clone(),
            library_program.clone(),
            executor_program_acc.clone(),
            executor_config_acc.clone(),
            price_feed_executor_acc.clone(),
            price_feed_config_executor_acc.clone(),
            dvn_program_acc.clone(),
            dvn_config_acc.clone(),
            price_feed_dvn_acc.clone(),
            price_feed_config_dvn_acc.clone(),
        ],
        signer_seeds,
    )?;

    msg!("SPL token deposit successful");

    Ok(())
}

fn validate_accounts(
    user: &AccountInfo,
    user_token_account: &AccountInfo,
    spl_tokens_vault_data_acc: &AccountInfo,
    spl_tokens_vault_acc: &AccountInfo,
    mint: &AccountInfo,
    token_program: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    messages_buffer_acc: &AccountInfo,
    detailed_messages_buffer_acc: &AccountInfo,
    twine_chain_role_manager_acc: &AccountInfo,
    twine_chain_program: &AccountInfo,
    twine_chain_storage_acc: &AccountInfo,
    messages_replicator_acc: &AccountInfo,
    system_program: &AccountInfo,
    layer_zero_info_acc: &AccountInfo,
    program_id: &Pubkey,
) -> Result<(TwineChainStorage, u64, u64), ProgramError> {
    if !user.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if user_token_account.owner != &spl_token::id() {
        msg!("Invalid user token account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    let (expected_spl_tokens_vault_data_acc, _) = derive_spl_tokens_vault_data(program_id);
    verify_derived_address(
        expected_spl_tokens_vault_data_acc,
        spl_tokens_vault_data_acc,
    )?;

    if spl_tokens_vault_acc.owner != &spl_token::id() {
        msg!("Invalid SPL tokens vault account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if mint.owner != &spl_token::id() {
        msg!("Invalid mint account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_program.key != &spl_token::id() {
        msg!("Invalid token program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    let (expecte_token_decimal_mapping, _) = derive_token_decimal_mappings(program_id);
    verify_derived_address(expecte_token_decimal_mapping, token_decimal_mappings_acc)?;

    let (expected_messages_buffer, _) = derive_messages_buffer(&twine_chain_program_id);
    verify_derived_address(expected_messages_buffer, messages_buffer_acc)?;

    let (expected_detailed_messages_buffer, _) =
        derive_detailed_messages_buffer(&twine_chain_program_id);
    verify_derived_address(
        expected_detailed_messages_buffer,
        detailed_messages_buffer_acc,
    )?;

    let (expected_token_decimal_mappings, _) = derive_token_decimal_mappings(program_id);
    verify_derived_address(expected_token_decimal_mappings, token_decimal_mappings_acc)?;

    let (expected_role_manager, _) = derive_twine_chain_role_manager(&twine_chain_program_id);
    verify_derived_address(expected_role_manager, twine_chain_role_manager_acc)?;

    let (expected_twine_chain_storage, _) = derive_twine_chain_storage(&twine_chain_program_id);
    verify_derived_address(expected_twine_chain_storage, twine_chain_storage_acc)?;

    verify_system_program(system_program)?;

    let (expected_layer_zero_pda, _) = derive_layer_zero_info(program_id);
    verify_derived_address(expected_layer_zero_pda, layer_zero_info_acc)?;

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
