use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
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

pub fn set_verification_status(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    verifcation_status: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let twine_operation_handler_acc = next_account_info(account_info_iter)?;

    // validate provided accounts
    let mut twine_chain_storage_data = validate_accounts(
        program_id,
        twine_chain_storage_acc,
        role_manager_acc,
        twine_operation_handler_acc,
    )?;

    twine_chain_storage_data.skip_verification = verifcation_status;

    // Update twine chain storage
    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Verification Status Updated");
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    twine_chain_storage_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
) -> Result<TwineChainStorage, ProgramError> {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let (expected_twine_chain_storage_pda, _) = derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    // Validate key
    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Deserialize account data
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Checks if signer has required role(TwineOperationHandler)
    if !role_manager_data.has_role(
        twine_operation_handler_acc.key,
        RoleType::TwineOperationHandler,
    ) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    // Deserialize Twine chain storage's data
    let twine_chain_storage_data =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(twine_chain_storage_data)
}
