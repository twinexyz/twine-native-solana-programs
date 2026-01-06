use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{TwineChainRoleManager, TwineChainStorage},
    },
    utils::{
        address_derivation::{
            derive_twine_chain_role_manager, derive_twine_chain_storage, verify_derived_address,
            verify_system_program,
        },
        constants::TWINE_CHAIN_STORAGE_PREFIX,
    },
};

pub fn empty_keccak256() -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update("");
    hasher.finalize().into()
}

pub fn initialize_chain_storage(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Dervive and validate PDA
    let twine_chain_storage_bump = validate_accounts(
        program_id,
        twine_chain_storage_acc,
        role_manager_acc,
        chain_admin_acc,
        system_program,
    )?;

    let rent = Rent::get()?;

    // Create account
    if twine_chain_storage_acc.data_is_empty() {
        let chain_storage_space = TwineChainStorage::LEN;
        let required_lamports = rent.minimum_balance(chain_storage_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            twine_chain_storage_acc.key,
            required_lamports,
            chain_storage_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                twine_chain_storage_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                TWINE_CHAIN_STORAGE_PREFIX.as_bytes(),
                &[twine_chain_storage_bump],
            ]],
        )?;
    }

    let twine_chain_storage_data = TwineChainStorage {
        is_initialized: true,
        last_copied_message_start_nonce: 0,
        last_copied_message_end_nonce: 0,
        last_committed_batch_number: 0,
        last_finalized_batch_number: 0,
        total_msg_handled_on_twine: 0,
        groth16_vk: Vec::new(),
        finalize_vkey: String::from(""),
        refund_vkey: String::from(""),
        forced_withdrawal_vkey: String::from(""),
        l2_withdrawal_vkey: String::from(""),
        skip_verification: true,
        last_committed_batch_hash: empty_keccak256(),
        last_finalized_batch_hash: empty_keccak256(),
    };

    twine_chain_storage_data
        .serialize(&mut &mut twine_chain_storage_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Twine Chain Storage Initialized");

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    twine_chain_storage_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<u8, ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_twine_chain_storage_pda, twine_chain_storage_bump) =
        derive_twine_chain_storage(program_id);
    verify_derived_address(expected_twine_chain_storage_pda, twine_chain_storage_acc)?;

    let (expected_role_manager_pda, _) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    verify_system_program(system_program)?;

    if !twine_chain_storage_acc.data_is_empty() {
        let twine_chain_data: TwineChainStorage =
            TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;

        if twine_chain_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // Checks if signer has required role(ChainAdmin)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if role_manager_data.chain_admin != *chain_admin_acc.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(twine_chain_storage_bump)
}