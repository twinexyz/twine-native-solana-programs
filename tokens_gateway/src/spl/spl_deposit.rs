use crate::core::error::ProgramCustomError;
use crate::core::state::{SplTokensVaultData, TokenDecimalMappings};
use crate::utils::constants::SPL_DATA_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    sysvar::Sysvar,
};
use spl_token::instruction as token_instruction;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use twine_chain::core::state::{DepositMessageInfo, DepositMessagesBuffer};
use twine_chain::utils::constants::DEPOSIT_BUFFER_PREFIX;

pub fn spl_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let deposit_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    if amount == 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if l1_token != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }
    let spl_data_seeds = &[SPL_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let user_token_data = TokenAccount::unpack(&user_token_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if user_token_data.amount < amount {
        msg!("User token account has insufficient funds");
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }
    let token_decimal_mappings =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;
    let decimal_mapping = token_decimal_mappings
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    let transfer_instruction = token_instruction::transfer(
        token_program.key,
        user_token_account.key,
        spl_tokens_vault_acc.key,
        user.key,
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
        SplTokensVaultData::try_from_slice(&spl_tokens_vault_data_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    spl_tokens_vault_data.update_deposit(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut *spl_tokens_vault_data_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

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
        from_l1_pubkey: user_token_account.key.to_string(),
        to_twine_address: receiver_twine_address,
        l1_token:l1_token,
        l2_token:l2_token,
        amount: l2_amount,
    };

    let discriminator: u8 = 5;
    let mut append_instruction_data = vec![discriminator];
    deposit_info
        .serialize(&mut &mut append_instruction_data[1..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let append_instruction_accounts = vec![
        AccountMeta::new(*deposit_messages_buffer_acc.key, false),
        AccountMeta::new_readonly(*role_manager_acc.key, false),
        AccountMeta::new_readonly(*spl_tokens_vault_data_acc.key, true),
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
            role_manager_acc.clone(),
            spl_tokens_vault_data_acc.clone(),
        ],
        &[&[SPL_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;

    msg!("SPL token deposit successful");
    Ok(())
}
