use borsh::{BorshDeserialize, BorshSerialize};
use crate::core::error::ProgramCustomError;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction,
};
use sp1_solana::verify_proof;
use crate::core::state::FinalizeInputWithdrawal;
use spl_token::instruction as token_instruction;

pub fn finalize_spl_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdrawal_inputs: FinalizeInputWithdrawal,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data = next_account_info(account_info_iter)?;
    let spl_tokens_vault = next_account_info(account_info_iter)?;
    let vault_authority = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let execution_message_buffer = next_account_info(account_info_iter)?;
    let receiver = next_account_info(account_info_iter)?;
    let token_decimal_mappings = next_account_info(account_info_iter)?;
    let clock = next_account_info(account_info_iter)?;
    let sp_verifier_program = next_account_info(account_info_iter)?;

    // Validate inputs
    if withdrawal_inputs.public_input.batch_number == 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if withdrawal_inputs.public_input.l1_token_address != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    msg!("SPL token withdrawal finalized successfully");
    Ok(())
}
