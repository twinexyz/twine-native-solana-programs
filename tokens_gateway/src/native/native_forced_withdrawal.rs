use crate::{
    core::{
        error::ProgramCustomError,
        state::{SignMessageInfo, TokenDecimalMappings},
    },
    utils::{
        address_derivation::derive_native_token_vault_data,
        constants::{CHAIN_ID, FORCED_WITHDRAW_TRANSACTION, NATIVE_TOKEN_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
        recover_address::recover_address,
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{DetailedMessagesBuffer, MessageInfo, TransactionType},
    },
    ID as twine_chain_program_id,
};

pub fn forced_native_token_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramError::InvalidArgument);
    }

    if !is_valid_ethereum_address(&from_twine_address)? {
        return Err(ProgramCustomError::InvalidAccount.into());
    }

    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if is_valid_ethereum_address(&to_l1_pubkey)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let _ = program_id;
    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_role_manager_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;
    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    validate_accounts(
        user_account,
        native_token_vault_data_acc,
        messages_buffer_acc,
        detailed_messages_buffer_acc,
        twine_chain_role_manager_acc,
        token_decimal_mappings_acc,
        twine_chain_program,
        program_id,
    )?;

    if messages_buffer_acc.owner != &twine_chain_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if twine_chain_role_manager_acc.owner != &twine_chain_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let token_decimal_mapping =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])?;

    let decimal_mapping = token_decimal_mapping
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    if (l2_token != decimal_mapping.l2_token.to_string()) {
        return Err(ProgramCustomError::TokenMappingNotFound.into());
    }

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    let forced_withdrawal_messages_buffer =
        DetailedMessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = forced_withdrawal_messages_buffer.message_nonce + 1;

    let clock = Clock::get()?;

    let withdraw_info = MessageInfo {
        txn_type: TransactionType::Withdraw,
        nonce: u64_nonce,
        chain_id: CHAIN_ID,
        slot_number: clock.slot,
        l1_pubkey: to_l1_pubkey,
        twine_address: from_twine_address.clone(),
        l1_token: l1_token,
        l2_token: l2_token,
        amount: l2_amount.to_string(),
        data: Vec::<u8>::new(),
    };

    let sign_info = SignMessageInfo {
        nonce: u64_nonce,
        chain_id: CHAIN_ID,
        amount: amount,
        l1_pubkey: withdraw_info.l1_pubkey.clone(),
        twine_address: withdraw_info.twine_address.clone(),
        l1_token: withdraw_info.l1_token.clone(),
        l2_token: withdraw_info.l2_token.clone(),
    };

    // Verify signature
    let recovered_address = recover_address(sign_info.clone(), signature)?;
    if recovered_address.to_lowercase() != withdraw_info.twine_address.to_lowercase() {
        return Err(ProgramCustomError::PublicKeyMismatch.into());
    }

    let (_, native_data_bump) = derive_native_token_vault_data(&program_id);

    let payload = TwineChainInstruction::AppendForcedWithdrawalMessage {
        withdraw_info: withdraw_info,
    };

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
        &[&[
            NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
            &[native_data_bump],
        ]],
    )?;

    Ok(())
}

fn validate_accounts(
    user_account: &AccountInfo,
    native_token_vault_data_acc: &AccountInfo,
    messages_buffer_acc: &AccountInfo,
    detailed_messages_buffer_acc: &AccountInfo,
    twine_chain_role_manager_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !user_account.is_signer {
        msg!("User account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if native_token_vault_data_acc.owner != program_id {
        msg!("Invalid native token vault data account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if messages_buffer_acc.owner != &twine_chain_program_id {
        msg!("Invalid forced withdrawal messages buffer account owner");
        return Err(ProgramError::IncorrectProgramId);
    }
    if detailed_messages_buffer_acc.owner != &twine_chain_program_id {
        msg!("Invalid forced withdrawal messages buffer account owner");
        return Err(ProgramError::IncorrectProgramId);
    }
    if twine_chain_role_manager_acc.owner != &twine_chain_program_id {
        msg!("Invalid role manager account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_decimal_mappings_acc.owner != program_id {
        msg!("Invalid token decimal mappings account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if twine_chain_program.key != &twine_chain_program_id {
        msg!("Invalid Twine chain program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}
