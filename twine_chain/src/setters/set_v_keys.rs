use crate::core::error::ProgramCustomError;
use crate::core::state::TwineChainStorage;
use crate::utils::constants::{TWINE_CHAIN_STORAGE_PREFIX, ROLE_MANAGER_PREFIX};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn set_v_keys(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    groth16_vk: Vec<u8>,
    execution_vkey: String,
    inclusion_vkey: String,
    withdrawal_vkey: String,
) -> ProgramResult {
    // TODO: Add role manager so that only TwineOperationHandler can call this function

    let account_info_iter = &mut accounts.iter();
    let twine_chain_storage = next_account_info(account_info_iter)?;
    let twine_operation_handler = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;

    // Validate signer
    if !twine_operation_handler.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate PDA
    let (expected_twine_chain_storage_pda, _twine_chain_storage_bump_seed) =
        Pubkey::find_program_address(&[TWINE_CHAIN_STORAGE_PREFIX.as_bytes()], program_id);
    if expected_twine_chain_storage_pda != *twine_chain_storage.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate data length
    if execution_vkey.len() > 32 || inclusion_vkey.len() > 32 || withdrawal_vkey.len() > 32 {
        return Err(ProgramCustomError::InvalidDataLength.into());
    }

    // Update account data
    let mut twine_chain_storage_data =
        TwineChainStorage::try_from_slice(&twine_chain_storage.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if twine chain storage is initialized
    if !twine_chain_storage_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    twine_chain_storage_data.groth16_vk = groth16_vk;
    twine_chain_storage_data.execution_vkey = execution_vkey;
    twine_chain_storage_data.inclusion_vkey = inclusion_vkey;
    twine_chain_storage_data.withdrawal_vkey = withdrawal_vkey;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}
