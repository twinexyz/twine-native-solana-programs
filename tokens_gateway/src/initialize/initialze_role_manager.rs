use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use crate::{
    core::state::{RoleType, TokensGatewayRoleManager},
    utils::{
        address_derivation::derive_gateway_role_manager,
        constants::{INITIAL_CHAIN_ADMIN, ROLE_MANAGER_ACCOUNT_SIZE, ROLE_MANAGER_PREFIX},
    },
};

pub fn initialize_role_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    validate_accounts(
        role_manager_acc,
        chain_admin_acc,
        system_program,
        program_id,
    )?;
    let rent = Rent::default();
    let (_, role_bump) = derive_gateway_role_manager(&program_id);

    let required_lamports = rent.minimum_balance(ROLE_MANAGER_ACCOUNT_SIZE);
    let create_ix = system_instruction::create_account(
        chain_admin_acc.key,
        role_manager_acc.key,
        required_lamports,
        ROLE_MANAGER_ACCOUNT_SIZE as u64,
        program_id,
    );
    invoke_signed(
        &create_ix,
        &[
            chain_admin_acc.clone(),
            role_manager_acc.clone(),
            system_program.clone(),
        ],
        &[&[ROLE_MANAGER_PREFIX.as_bytes(), &[role_bump]]],
    )?;

    let chain_admin: Pubkey = INITIAL_CHAIN_ADMIN
        .parse()
        .map_err(|_| ProgramError::InvalidArgument)?;

    let role_manager = TokensGatewayRoleManager {
        is_initialized: true,
        chain_admin,
        roles: vec![(chain_admin, RoleType::TwineOperationHandler)],
    };

    let mut data = role_manager_acc.data.borrow_mut();
    role_manager.serialize(&mut &mut data[..])?;

    msg!(
        "EVENT:RoleManagerInitialized: role_manager={}, chain_admin={}, initial_role={}",
        role_manager_acc.key,
        chain_admin,
        "TwineOperationHandler"
    );

    Ok(())
}

fn validate_accounts(
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.key != &solana_program::system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check if account is already initialized
    {
        let data = role_manager_acc.data.borrow();
        if !data.is_empty() {
            let mut data_slice = &data[..];
            if let Ok(role_manager) = TokensGatewayRoleManager::deserialize(&mut data_slice) {
                if role_manager.is_initialized {
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
            }
        }
    }

    let (expected_role_manager_key, _) = derive_gateway_role_manager(&program_id);

    if expected_role_manager_key != *role_manager_acc.key {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}