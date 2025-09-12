use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{RoleType, TwineChainRoleManager, TwineChainStorage},
    },
    utils::address_derivation::{
        derive_twine_chain_role_manager, derive_twine_chain_storage, verify_derived_address,
    },
};

pub fn set_v_keys(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    groth16_vk: Vec<u8>,
    execution_vkey: String,
    inclusion_vkey: String,
    withdrawal_vkey: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let twine_operation_handler_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        twine_chain_storage_acc,
        twine_operation_handler_acc,
        role_manager_acc,
    )?;

    // Validate data length [2(0x) + 32Bytes(64 hex char) = 66 hex characters]
    if execution_vkey.len() > 66 || inclusion_vkey.len() > 66 || withdrawal_vkey.len() > 66 {
        return Err(ProgramCustomError::InvalidDataLength.into());
    }

    // Update account data
    let mut twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    twine_chain_storage_data.groth16_vk = groth16_vk;
    twine_chain_storage_data.execution_vkey = execution_vkey;
    twine_chain_storage_data.inclusion_vkey = inclusion_vkey;
    twine_chain_storage_data.withdrawal_vkey = withdrawal_vkey;

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    twine_chain_storage_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_twine_chain_storage_pda, _twine_chain_storage_bump_seed) =
        derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Checks if signer has required role(TwineOperationHandler)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(
        twine_operation_handler_acc.key,
        RoleType::TwineOperationHandler,
    ) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    Ok(())
}

