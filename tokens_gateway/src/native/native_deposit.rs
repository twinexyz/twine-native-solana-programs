use crate::core::error::ProgramCustomError;
use crate::core::state::{NativeTokenVaultData, TokenDecimalMappings};
use crate::utils::constants::NATIVE_DATA_PREFIX;
use twine_chain::utils::constants::DEPOSIT_BUFFER_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use twine_chain::core::state::{DepositMessageInfo, DepositMessagesBuffer};

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
// Handles native token (SOL) deposits
pub fn native_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }
    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }
    let account_info_iter = &mut accounts.iter();

    // Account[0]: the depositor (user) - must be a signer
    let user_account = next_account_info(account_info_iter)?;
    let native_vault_account = next_account_info(account_info_iter)?;
    let native_vault_data_account = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let deposit_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if user_account.lamports() < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    if native_vault_data_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let transfer_ix =
        system_instruction::transfer(user_account.key, native_vault_account.key, amount);
    invoke(
        &transfer_ix,
        &[
            user_account.clone(),
            native_vault_account.clone(),
            system_program_info.clone(),
        ],
    )?;

    let mut vault_data =
        NativeTokenVaultData::try_from_slice(&native_vault_data_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    vault_data
        .serialize(&mut *native_vault_data_account.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let token_decimal_mapping =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;

    let decimal_mapping = token_decimal_mapping
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    let (expected_deposit_pda, _) =
        Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_deposit_pda != *deposit_messages_buffer_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let deposit_message_buffer =
        DepositMessagesBuffer::try_from_slice(&deposit_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = deposit_message_buffer.deposit_nonce + 1;

    let clock = Clock::get()?;

    let deposit_info = DepositMessageInfo {
        nonce: u64_nonce,
        chain_id: 900,
        slot_number: clock.slot,
        from_l1_pubkey: user_account.key.to_string(),
        to_twine_address: receiver_twine_address,
        l1_token,
        l2_token,
        amount: l2_amount,
    };

    let native_data_seeds = &[NATIVE_DATA_PREFIX.as_bytes()];
    let (_, native_data_bump) = Pubkey::find_program_address(native_data_seeds, program_id);

    let discriminator: u8 = 5;

    let mut append_instruction_data = vec![discriminator];
    deposit_info
        .serialize(&mut &mut append_instruction_data[1..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let append_instruction_accounts = vec![
        AccountMeta::new(*deposit_messages_buffer_acc.key, false),
        AccountMeta::new_readonly(*role_manager.key, false),
        AccountMeta::new_readonly(*native_vault_data_account.key, true),
    ];
    let append_instruction = Instruction {
        program_id: *twine_chain_program.key,
        accounts: append_instruction_accounts,
        data: append_instruction_data,
    };

    invoke_signed(
        &append_instruction,
        &[
            deposit_messages_buffer_acc.clone(),
            role_manager.clone(),
            native_vault_data_account.clone(),
        ],
        &[&[NATIVE_DATA_PREFIX.as_bytes(), &[native_data_bump]]],
    )?;

    msg!(
        "Native token deposit successful: {} lamports deposited",
        amount
    );

    Ok(())
}
