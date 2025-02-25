use crate::core::state::TwineChainRoleManager;
use crate::core::error::ProgramCustomError;
use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES, ROLE_MANAGER_PREFIX};
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

pub fn initialize_role_manager(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;
    if !chain_admin_acc.is_signer {
        return Err(ProgramCustomError::InvalidSigner.into());
    }
    let rent = Rent::get()?;
    
    let role_manager_space = 8 + 32 + 4 + (MAX_ROLES * 33);

    let (expected_pda, bump) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_pda != *role_manager_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
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
            &[&[ROLE_MANAGER_PREFIX.as_bytes(), &[bump]]],
        )?;
    }

    let mut role_manager = TwineChainRoleManager::try_from_slice(&role_manager_acc.data.borrow())
    .map_err(|_| {
        ProgramError::InvalidAccountData
    })?;

    let chain_admin: Pubkey = INITIAL_CHAIN_ADMIN.parse().expect("Invalid Pubkey");
    role_manager.chain_admin = chain_admin;
    role_manager.twine_operator = Pubkey::default();
    role_manager.token_gateway_program = Pubkey::default();
    role_manager.serialize(&mut *role_manager_acc.data.borrow_mut()).map_err(|_| {
        msg!("Failed to serialize updated role manager state");
        ProgramCustomError::SerializeFailed 
    })?;

    Ok(())
}
