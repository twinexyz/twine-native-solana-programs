use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::Clock,
    system_instruction,
    sysvar::Sysvar,
};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{DepositMessageInfo, MessagesBuffer, TransactionType},
    },
    utils::constants::{DEPOSIT_MESSAGE_TYPE, MESSAGES_BUFFER_PREFIX},
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{NativeTokenVaultData, TokenDecimalMappings},
    },
    utils::{
        address_derivation::derive_native_token_vault_data,
        constants::{DEPOSIT_TRANSACTION, NATIVE_TOKEN_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

// Handles native token (SOL) deposits
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
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let twine_chain_role_manager_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    validate_accounts(
        user_account,
        native_token_vault_acc,
        native_token_vault_data_acc,
        token_decimal_mappings_acc,
        twine_chain_role_manager_acc,
        system_program,
        twine_chain_program,
        program_id,
    )?;

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
    let token_decimal_mappings_data =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let decimal_mapping = token_decimal_mappings_data
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

    let (expected_deposit_pda, _) = Pubkey::find_program_address(
        &[MESSAGES_BUFFER_PREFIX.as_bytes()],
        &twine_chain_program_id,
    );

    if expected_deposit_pda != *messages_buffer_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let deposit_message_buffer =
        MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = deposit_message_buffer.message_nonce + 1;

    let clock = Clock::get()?;

    let deposit_info = DepositMessageInfo {
        txn_type: TransactionType::Deposit,
        nonce: u64_nonce,
        chain_id: 900,
        slot_number: clock.slot,
        from_l1_pubkey: user_account.key.to_string(),
        to_twine_address: receiver_twine_address,
        l1_token,
        l2_token,
        amount: l2_amount,
        data,
    };

    let payload = TwineChainInstruction::AppendDepositMessage {
        deposit_info: deposit_info,
    };

    let mut append_instruction_data = vec![];

    append_instruction_data.extend(payload.try_to_vec().unwrap());

    let append_instruction_accounts = vec![
        AccountMeta::new(*messages_buffer_acc.key, false),
        AccountMeta::new(*twine_chain_role_manager_acc.key, false),
        AccountMeta::new_readonly(*native_token_vault_data_acc.key, true),
    ];
    if twine_chain_program.key != &twine_chain_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let append_instruction = Instruction {
        program_id: *twine_chain_program.key,
        accounts: append_instruction_accounts,
        data: append_instruction_data,
    };

    let (_, native_data_bump) = derive_native_token_vault_data(&program_id);
    let seeds = &[
        NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
        &[native_data_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    invoke_signed(
        &append_instruction,
        &[
            messages_buffer_acc.clone(),
            twine_chain_role_manager_acc.clone(),
            native_token_vault_data_acc.clone(),
            twine_chain_program.clone(),
        ],
        signer_seeds,
    )?;

    Ok(())
}

fn validate_accounts(
    user: &AccountInfo,
    native_token_vault_acc: &AccountInfo,
    native_token_vault_data_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    system_program: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if native_token_vault_data_acc.owner != program_id {
        msg!("Invalid native token vault data account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_decimal_mappings_acc.owner != program_id {
        msg!("Invalid token decimal mappings account owner");
        return Err(ProgramError::IncorrectProgramId);
    }
    if role_manager_acc.owner != &twine_chain_program_id {
        msg!("Invalid role manager account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if twine_chain_program.key != &twine_chain_program_id {
        msg!("Invalid Twine chain program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Validate system program
    if system_program.key != &solana_program::system_program::ID {
        msg!("Invalid system program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}