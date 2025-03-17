use crate::core::state::TokensGatewayRoleManager;
use crate::utils::constants::{
    INITIAL_CHAIN_ADMIN, ROLE_MANAGER_PREFIX,MAX_ROLES
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn initialize_role_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Initializing Role Manager");
    let account_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let rent = Rent::get()?;
    let role_manager_seeds = &[ROLE_MANAGER_PREFIX.as_bytes()];
    let (expected_role_manager_key, role_bump) =
        Pubkey::find_program_address(role_manager_seeds, program_id);
    if expected_role_manager_key != *role_manager_acc.key {
        msg!("Role Manager PDA key mismatch");
        return Err(ProgramError::InvalidAccountData.into());
    }

    let role_manager_space = 8 + 32 + 4 + (MAX_ROLES * 33);
    if role_manager_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(role_manager_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            role_manager_acc.key,
            required_lamports,
            role_manager_space as u64,
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
    }

    let mut role_manager =
        TokensGatewayRoleManager::try_from_slice(&role_manager_acc.data.borrow())?;
    let chain_admin: Pubkey = INITIAL_CHAIN_ADMIN
        .parse()
        .map_err(|_| ProgramError::InvalidArgument)?;
    role_manager.chain_admin = chain_admin;
    role_manager.roles.clear();
    role_manager.serialize(&mut *role_manager_acc.data.borrow_mut())?;
    msg!("Role Manager initialized successfully");
    Ok(())
}
