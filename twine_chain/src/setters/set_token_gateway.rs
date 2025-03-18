use crate::core::error::ProgramCustomError;
use crate::core::state::TwineChainRoleManager;
use crate::utils::constants::ROLE_MANAGER_PREFIX;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn set_token_gateway(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_gateway_program: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager = next_account_info(account_info_iter)?;
    let twine_operation_handler = next_account_info(account_info_iter)?;

    // Validate signer
    if !twine_operation_handler.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and Validate PDA
    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Deserialize account data
    let mut role_manager_data = TwineChainRoleManager::try_from_slice(&role_manager.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !role_manager_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    // Update Token Gateway value
    role_manager_data.token_gateway_program = token_gateway_program;

    role_manager_data
        .serialize(&mut &mut role_manager.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Token Gateway Updated");
    Ok(())
}
