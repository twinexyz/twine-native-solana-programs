use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    core::error::ProgramCustomError,
    core::state::{RoleType, TokensGatewayRoleManager},
};

pub fn set_role_chain_admin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_admin: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let mut role_manager =
        validate_accounts_and_deserialize(program_id, role_manager_acc, chain_admin_acc)?;
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
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let mut role_manager =
        validate_accounts_and_deserialize(program_id, role_manager_acc, chain_admin_acc)?;
    if role_manager.chain_admin != *chain_admin_acc.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    role_manager.roles.push((address, role));
    role_manager
        .serialize(&mut &mut role_manager_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!(
        "EVENT:RoleAdded: address={}, role={:?}, added_by={}",
        address,
        role,
        chain_admin_acc.key
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
    let role_manager_acc = next_account_info(account_info_iter)?;
    let authority_acc = next_account_info(account_info_iter)?;
    let mut role_manager =
        validate_accounts_and_deserialize(program_id, role_manager_acc, authority_acc)?;

    if !role_manager.has_role(&authority_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    if !role_manager.remove_role(&address, role) {
        return Err(ProgramCustomError::RemoveFailed.into());
    }
    role_manager
        .serialize(&mut &mut role_manager_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!(
        "EVENT:RoleRemoved: address={}, role={:?}, removed_by={}",
        address,
        role,
        authority_acc.key
    );

    Ok(())
}

pub fn validate_accounts_and_deserialize(
    program_id: &Pubkey,
    role_manager_account: &AccountInfo,
    authority_account: &AccountInfo,
) -> Result<TokensGatewayRoleManager, ProgramError> {
    if role_manager_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !authority_account.is_signer {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    let role_manager =
        TokensGatewayRoleManager::deserialize(&mut &role_manager_account.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(role_manager)
}
