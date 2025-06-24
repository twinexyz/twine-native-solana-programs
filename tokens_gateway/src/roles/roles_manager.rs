use crate::{
    core::error::ProgramCustomError,
    core::state::{RoleType, TokensGatewayRoleManager},
};
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
    let account_info_iter = &mut accounts.iter();

    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;

    if role_manager_acc.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !chain_admin_acc.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    let mut role_manager =
        TokensGatewayRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])?;
    if role_manager.chain_admin != *chain_admin_acc.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    let old_man = role_manager.chain_admin;
    role_manager.chain_admin = new_admin;
    role_manager.serialize(&mut &mut role_manager_acc.data.borrow_mut()[..])?;

    msg!(
        "EVENT:ChainAdminUpdated: old_admin={}, new_admin={}, updated_by={}",
        old_man,
        new_admin,
        chain_admin_acc.key
    );

    Ok(())
}

pub fn add_role(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    address: Pubkey,
    role: RoleType,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager_info = next_account_info(account_info_iter)?;
    let chain_admin_info = next_account_info(account_info_iter)?;

    if role_manager_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !chain_admin_info.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    let mut role_manager =
        TokensGatewayRoleManager::deserialize(&mut &role_manager_info.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if role_manager.chain_admin != *chain_admin_info.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    role_manager.roles.push((address, role));

    role_manager
        .serialize(&mut &mut role_manager_info.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!(
        "EVENT:RoleAdded: address={}, role={:?}, added_by={}",
        address,
        role,
        chain_admin_info.key
    );

    Ok(())
}

pub fn remove_role(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    address: Pubkey,
    role: RoleType,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    if role_manager_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !authority_info.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    let mut role_manager =
        TokensGatewayRoleManager::deserialize(&mut &role_manager_info.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager.has_role(&authority_info.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    if !role_manager.remove_role(&address, role) {
        return Err(ProgramCustomError::RemoveFailed.into());
    }
    role_manager
        .serialize(&mut &mut role_manager_info.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!(
        "EVENT:RoleRemoved: address={}, role={:?}, removed_by={}",
        address,
        role,
        authority_info.key
    );

    Ok(())
}
