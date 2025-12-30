use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHex;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{LayerZeroInfo, RoleType, TwineChainRoleManager},
    },
    utils::address_derivation::{
        derive_layer_zero_info, derive_twine_chain_role_manager, verify_derived_address,
    },
};

pub fn set_lz_info(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    dst_eid: u32,
    dst_oapp_address: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let layer_zero_info_acc = next_account_info(account_info_iter)?;
    let twine_operation_handler_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        layer_zero_info_acc,
        twine_operation_handler_acc,
        role_manager_acc,
    )?;

    let mut layer_zero_info_data =
        LayerZeroInfo::deserialize(&mut &layer_zero_info_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    layer_zero_info_data.receiver = evm_address_to_bytes32(dst_oapp_address);
    layer_zero_info_data.dst_eid = dst_eid;

    layer_zero_info_data
        .serialize(&mut &mut layer_zero_info_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}

fn evm_address_to_bytes32(addr: String) -> [u8; 32] {
    let cleaned = addr.trim_start_matches("0x");
    let raw_bytes: [u8; 20] =
        <[u8; 20]>::from_hex(cleaned).expect("Invalid EVM address hex string");

    // Left-pad the 20 bytes to 32 bytes
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(&raw_bytes);
    padded
}

fn validate_accounts(
    program_id: &Pubkey,
    layer_zero_info_acc: &AccountInfo,
    twine_operation_handler_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !twine_operation_handler_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_layer_zero_info, _) = derive_layer_zero_info(program_id);
    verify_derived_address(expected_layer_zero_info, layer_zero_info_acc)?;

    let (expected_role_manager_pda, _role_manager_bump_seed) =
        derive_twine_chain_role_manager(program_id);
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
