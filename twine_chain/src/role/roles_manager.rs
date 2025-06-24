use crate::core::error::ProgramCustomError;
use crate::core::state::{RoleType, TwineChainRoleManager};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn set_role_chain_admin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_admin: Pubkey,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let role_manager_info = next_account_info(account_iter)?;
    let chain_admin_info = next_account_info(account_iter)?;

    if role_manager_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !chain_admin_info.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    let mut role_manager = TwineChainRoleManager::try_from_slice(&role_manager_info.data.borrow())?;
    if role_manager.chain_admin != *chain_admin_info.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    role_manager.serialize(&mut *role_manager_info.data.borrow_mut())?;
    msg!("Chain admin updated successfully.");
    role_manager.chain_admin = new_admin;

    Ok(())
}
pub fn add_role(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    address: Pubkey,
    role: RoleType,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let role_manager_info = next_account_info(account_iter)?;
    let chain_admin_info = next_account_info(account_iter)?;
    if role_manager_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !chain_admin_info.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    let mut role_manager =
        TwineChainRoleManager::deserialize(&mut &role_manager_info.data.borrow()[..])?;
    role_manager.roles.push((address, role));
    role_manager.serialize(&mut *role_manager_info.data.borrow_mut())?;

    msg!("Role added successfully.");
    Ok(())
}
pub fn remove_role(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    address: Pubkey,
    role: RoleType,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let role_manager_info = next_account_info(account_iter)?;
    let authority_info = next_account_info(account_iter)?;

    if role_manager_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !authority_info.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    let mut role_manager = TwineChainRoleManager::try_from_slice(&role_manager_info.data.borrow())?;
    if !role_manager.has_role(&authority_info.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    let removed = role_manager.remove_role(&address, role);
    if !removed {
        return Err(ProgramCustomError::RemoveFailed.into());
    }
    Ok(())
}
