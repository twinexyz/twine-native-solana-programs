use crate::{core::error::ProgramCustomError, utils::constants::*};

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn derive_native_token_vault(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_PREFIX.as_bytes()], &program_id)
}

pub fn derive_native_token_vault_data(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes()], &program_id)
}

pub fn derive_spl_tokens_vault_data(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()], &program_id)
}

pub fn derive_spl_vault_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SPL_AUTH_PREFIX.as_bytes()], &program_id)
}

pub fn derive_token_decimal_mappings(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_DECIMAL_MAPPINGS_PREFIX.as_bytes()], &program_id)
}

pub fn derive_gateway_role_manager(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], &program_id)
}

pub fn derive_executed_withdrawals_pda(program_id: &Pubkey, messge_nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            EXECUTED_WITHDRAWALS_PREFIX.as_bytes(),
            &messge_nonce.to_be_bytes(),
        ],
        &program_id,
    )
}

pub fn derive_executed_payouts_pda(program_id: &Pubkey, messge_nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            EXECUTED_PAYOUTS_PREFIX.as_bytes(),
            &messge_nonce.to_be_bytes(),
        ],
        &program_id,
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
