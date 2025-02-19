use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use crate::utils::constants::{ROLE_MANAGER_PREFIX, INITIAL_CHAIN_ADMIN};

pub fn initialize_role_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Initializing Role Manager");
    let account_iter = &mut accounts.iter();
    let role_manager_info = next_account_info(account_iter)?;
    let signer_info = next_account_info(account_iter)?;

    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if role_manager_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    use crate::core::state::TokensGatewayRoleManager;

    let mut role_manager = TokensGatewayRoleManager::try_from_slice(&role_manager_info.data.borrow())?;
    let chain_admin: Pubkey = INITIAL_CHAIN_ADMIN.parse().map_err(|_| ProgramError::InvalidArgument)?;
    role_manager.chain_admin = chain_admin;
    role_manager.roles.clear();

    role_manager.serialize(&mut *role_manager_info.data.borrow_mut())?;
    msg!("Role Manager initialized successfully");
    Ok(())
}