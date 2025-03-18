use crate::core::error::ProgramCustomError;
use crate::core::state::{BatchPdaAccount, BlockInfo};
use crate::utils::constants::{COMMITMENT_PDA_PREFIX, ROLE_MANAGER_PREFIX};
use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::program_pack::IsInitialized;
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

pub fn initialize_genesis_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    genesis_block_hash: [u8; 32],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let first_batch = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let initializer_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    let first_batch_space = 1 + (4 + BlockInfo::LEN) + 1 + 1;

    let rent = Rent::get()?;

    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Dervive and validate Commitment PDA
    let (expected_commitment_pda, genesis_batch_bump) = Pubkey::find_program_address(
        &[
            COMMITMENT_PDA_PREFIX.as_bytes(),
            &(0u64).to_be_bytes(),
            &(0u64).to_be_bytes(),
        ],
        program_id,
    );
    if expected_commitment_pda != *first_batch.key {
        return Err(ProgramError::InvalidArgument);
    }

    // Dervive and validate RoleManager PDA
    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Create Genesis Batch PDA 
    if first_batch.data_is_empty() {
        let required_lamports = rent.minimum_balance(first_batch_space);
        let create_ix = system_instruction::create_account(
            initializer_acc.key,
            first_batch.key,
            required_lamports,
            first_batch_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                initializer_acc.clone(),
                first_batch.clone(),
                system_program.clone(),
            ],
            &[&[
                COMMITMENT_PDA_PREFIX.as_bytes().as_ref(),
                &[genesis_batch_bump],
            ]],
        )?;
    }

    // Deserialize and update account data
    let mut account_data = BatchPdaAccount::try_from_slice(&first_batch.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if account_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    account_data.is_initialized = true;
    account_data.infos.push(BlockInfo {
        previous_hash: [0u8; 32],
        block_hash: genesis_block_hash,
        transaction_root: [0u8; 32],
        receipt_root: [0u8; 32],
    });
    account_data.verified = true;
    account_data.is_full = true;

    account_data
        .serialize(&mut &mut first_batch.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Genesis Batch Initialized");

    Ok(())
}
