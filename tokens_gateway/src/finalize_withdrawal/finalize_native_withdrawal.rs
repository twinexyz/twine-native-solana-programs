use borsh::{BorshDeserialize, BorshSerialize};
use crate::core::error::ProgramCustomError;
use crate::core::state::FinalizeInputWithdrawal;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction,
};
use sp1_solana::verify_proof;

pub fn finalize_native_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdrawal_inputs: FinalizeInputWithdrawal,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user = next_account_info(account_info_iter)?;
    let native_token_vault = next_account_info(account_info_iter)?;
    let native_token_vault_data = next_account_info(account_info_iter)?;
    let receiver = next_account_info(account_info_iter)?;
    let token_decimal_mappings = next_account_info(account_info_iter)?;
    let execution_message_buffer = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let sp_verifier_program = next_account_info(account_info_iter)?;

    // Validate inputs
    msg!("Validating withdrawal inputs");
    if withdrawal_inputs.public_input.batch_number < 0  {
        return Err( ProgramCustomError::InvalidArgument.into());
    }
    // let amount = TokenDecimalMappings::parse_amount_to_u64(&withdrawal_inputs.public_input.amount)?;
    // if amount <= 0 {
    //     return Err(ProgramCustomError::InvalidL1Token.into());
    // }
  
    if withdrawal_inputs.public_input.l1_token_address == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }
    Ok(())
}
