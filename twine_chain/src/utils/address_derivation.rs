use crate::core::error::ProgramCustomError;
use crate::utils::constants::*;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn derive_deposit_message_buffer(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id)
}

pub fn derive_forced_withdraw_message_buffer(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes()], program_id)
}

pub fn derive_execution_message_buffer(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EXECUTION_BUFFER_PREFIX.as_bytes()], program_id)
}

pub fn derive_layer_zero_message_buffer(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[LAYER_ZERO_BUFFER_PREFIX.as_bytes()], program_id)
}

pub fn derive_role_manager(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id)
}

pub fn derive_twine_chain_storage(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TWINE_CHAIN_STORAGE_PREFIX.as_bytes()], program_id)
}

pub fn derive_commitment_pda(
    program_id: &Pubkey,
    start_block: u64,
    end_block: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            COMMITMENT_PDA_PREFIX.as_bytes(),
            &start_block.to_be_bytes(),
            &end_block.to_be_bytes(),
        ],
        program_id,
    )
}

pub fn verify_derived_address(
    derived_address: Pubkey,
    provided_address: &AccountInfo,
) -> ProgramResult {
    if derived_address != *provided_address.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    Ok(())
}

pub fn verify_system_program(provided_system_program: &AccountInfo) -> ProgramResult {
    if provided_system_program.key != &solana_program::system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub fn verify_owner(provided_address: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
    if provided_address.owner != expected_owner {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    Ok(())
}
