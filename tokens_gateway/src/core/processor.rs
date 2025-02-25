use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

use crate::core::error::ProgramCustomError;
use crate::core::instruction::{unpack_instruction, GatewayInstruction};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Parse the instruction.
    let instruction = unpack_instruction(instruction_data)
        .map_err(|_| ProgramCustomError::InvalidInstructionData)?;

    match instruction {
        GatewayInstruction::Initialize => {
            msg!("Processing Initialize instruction");
            // Manually fetch and validate accounts.
            let account_info_iter = &mut accounts.iter();
            let state_account = next_account_info(account_info_iter)?;
            Ok(())
        }
    }
}
