use crate::core::error::ProgramCustomError;
use crate::core::state::TokenDecimalMappings;
use crate::utils::constants::SPL_DATA_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use crate::utils::recover_address::recover_address;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use twine_chain::core::state::{ForcedWithdrawMessageInfo, ForcedWithdrawMessagesBuffer};

pub fn forced_spl_token_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_account = next_account_info(account_info_iter)?;
    let to_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let forced_withdrawal_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    // Perform necessary checks
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if !is_valid_ethereum_address(&from_twine_address)? {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if role_manager_acc.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    let spl_data_seeds = &[SPL_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

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

    let forced_withdrawal_messages_buffer = ForcedWithdrawMessagesBuffer::try_from_slice(
        &forced_withdrawal_messages_buffer_acc.data.borrow(),
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = forced_withdrawal_messages_buffer.withdraw_nonce + 1;

    let clock = Clock::get()?;

    let withdraw_info = ForcedWithdrawMessageInfo {
        nonce: u64_nonce,
        chain_id: 900,
        slot_number: clock.slot,
        from_twine_address: from_twine_address,
        to_l1_pubkey: to_l1_pubkey,
        l1_token: l1_token,
        l2_token: l2_token,
        amount: l2_amount.to_string(),
    };
    
    let recovered_address = recover_address(withdraw_info.clone(), signature)?;
    if recovered_address != withdraw_info.from_twine_address {
        return Err(ProgramError::InvalidArgument);
    }

    let discriminator: u8 = 6;

    let mut append_instruction_data = vec![discriminator];

    withdraw_info
        .serialize(&mut &mut append_instruction_data[1..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let append_instruction_accounts = vec![
        AccountMeta::new(*forced_withdrawal_messages_buffer_acc.key, false),
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
            forced_withdrawal_messages_buffer_acc.clone(),
            role_manager_acc.clone(),
            spl_tokens_vault_data_acc.clone(),
        ],
        &[&[SPL_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;
    Ok(())
}
