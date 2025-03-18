use crate::core::error::ProgramCustomError;
use crate::core::state::TwineChainStorage;
use crate::utils::constants::{TWINE_CHAIN_STORAGE_PREFIX, ROLE_MANAGER_PREFIX};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn initialize_chain_storage(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let chain_storage_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;
    let rent = Rent::get()?;
    let chain_storage_space = 8 + 700;

    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Dervive and validate PDA
    let (expected_twine_chain_storage_pda, bump) =
        Pubkey::find_program_address(&[TWINE_CHAIN_STORAGE_PREFIX.as_bytes()], program_id);
    if expected_twine_chain_storage_pda != *chain_storage_acc.key {
        return Err(ProgramError::InvalidArgument);
    }

    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    if chain_storage_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(chain_storage_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            chain_storage_acc.key,
            required_lamports,
            chain_storage_space as u64,
            program_id,
        );

        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                chain_storage_acc.clone(),
                system_program.clone(),
            ],
            &[&[TWINE_CHAIN_STORAGE_PREFIX.as_bytes(), &[bump]]],
        )?;
    }

    let mut twine_chain_storage =
        TwineChainStorage::try_from_slice(&chain_storage_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if twine_chain_storage.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Deserialize and update account data
    twine_chain_storage.is_initialized = true;

    twine_chain_storage.last_committed_batch.start_block = String::from("0");
    twine_chain_storage.last_committed_batch.end_block = String::from("0");

    twine_chain_storage.last_finalized_batch.start_block = String::from("0");
    twine_chain_storage.last_finalized_batch.end_block = String::from("0");

    twine_chain_storage.groth16_vk = Vec::new();
    twine_chain_storage.execution_vkey = String::from("");
    twine_chain_storage.inclusion_vkey = String::from("");
    twine_chain_storage.withdrawal_vkey = String::from("");

    twine_chain_storage
        .serialize(&mut *chain_storage_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}
