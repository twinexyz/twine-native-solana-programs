use crate::utils::constants::*;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

pub fn derive_native_token_vault(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_PREFIX.as_bytes()], program_id)
}

pub fn derive_native_token_vault_data(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes()], program_id)
}

pub fn derive_spl_tokens_vault_data(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()], program_id)
}

pub fn derive_spl_vault_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SPL_AUTH_PREFIX.as_bytes()], program_id)
}

pub fn derive_token_decimal_mappings(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_DECIMAL_MAPPINGS_PREFIX.as_bytes()], program_id)
}

pub fn derive_role_manager(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id)
}

pub fn derive_executed_withdrawals_buffer(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EXECUTED_WITHDRAWALS_BUFFER_PREFIX.as_bytes()], program_id)
}

pub fn verify_derived_address(
    derived_address: &Pubkey,
    provided_address: &Pubkey,
) -> Result<(), ProgramError> {
    if derived_address != provided_address {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
