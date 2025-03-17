use crate::core::error::ProgramCustomError;
use crate::core::state::{ExecutedWithdrawals, FinalizeInputWithdrawal, TokenDecimalMappings};
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use sp1_solana::verify_proof;
use spl_token::instruction as token_instruction;

pub fn finalize_spl_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdrawal_inputs: FinalizeInputWithdrawal,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user = next_account_info(account_info_iter)?;
    let mut spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let mut spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let mut vault_authority_acc = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let execution_message_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let clock = next_account_info(account_info_iter)?;
    let sp_verifier_program = next_account_info(account_info_iter)?;

    // Validate inputs
    if withdrawal_inputs.public_input.batch_number == 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if withdrawal_inputs.public_input.l1_token_address != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if withdrawal_inputs.public_input.l1_token_address == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&withdrawal_inputs.public_input.l2_token_address)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if withdrawal_inputs.public_input.l1_receiver_address != receiver_acc.key.to_string() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let token_decimal_mappings =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;
    let decimal_mapping = token_decimal_mappings
        .get_mapping(&withdrawal_inputs.public_input.l1_token_address)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;
    let converted_amount = TokenDecimalMappings::convert_l2_to_l1(
        &withdrawal_inputs.public_input.amount,
        decimal_mapping.l2_decimals,
        decimal_mapping.l1_decimals,
    )?;
    let actual_amount = TokenDecimalMappings::parse_amount_to_u64(&converted_amount)?;

    process_spl_token_withdrawal(
        &program_id,
        &spl_tokens_vault_acc,
        &spl_tokens_vault_data_acc,
        &vault_authority_acc,
        &mint,
        &token_program,
        &receiver_acc,
        actual_amount,
    )?;
    msg!("SPL token withdrawal finalized successfully");
    Ok(())
}

fn process_spl_token_withdrawal<'info>(
    program_id: &Pubkey,
    spl_tokens_vault: &AccountInfo<'info>,
    spl_tokens_vault_data: &AccountInfo<'info>,
    vault_authority: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    receiver: &AccountInfo<'info>,
    amount: u64,
) -> ProgramResult {
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    let vault_token_account = spl_token::state::Account::unpack(&spl_tokens_vault.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    if vault_token_account.amount < amount {
        return Err(ProgramError::InsufficientFunds);
    }
    Ok(())
}
