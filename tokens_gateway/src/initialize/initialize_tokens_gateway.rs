use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::core::state::{NativeTokenVaultData, SplTokensVaultData, TokenDecimalMappings};


pub fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    // Expected accounts in order:
    // [0] chain_admin (signer)
    // [1] native_token_vault_data (writable, owner = program_id)
    // [2] native_token_vault (writable, system account – PDA)
    // [3] spl_tokens_vault_data (writable, owner = program_id)
    // [4] token_decimal_mappings (writable, owner = program_id)
    // [5] role_manager (used for further validations; not used here)
    // [6] system_program
    let chain_admin = next_account_info(account_iter)?;
    let native_token_vault_data = next_account_info(account_iter)?;
    let native_token_vault = next_account_info(account_iter)?;
    let spl_tokens_vault_data = next_account_info(account_iter)?;
    let token_decimal_mappings = next_account_info(account_iter)?;
    let _role_manager = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Verify that the chain admin is a signer.
    if !chain_admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // Check account owner for state accounts.
    if native_token_vault_data.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if spl_tokens_vault_data.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if token_decimal_mappings.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    let mut native_vault_data = if native_token_vault_data.data_len() == 0 {
        NativeTokenVaultData { total_deposits: 0 }
    } else {
        NativeTokenVaultData::try_from_slice(&native_token_vault_data.data.borrow())?
    };
    native_vault_data.total_deposits = 0;
    native_vault_data.serialize(&mut *native_token_vault_data.data.borrow_mut())?;

    // Initialize and serialize the SPL Tokens Vault Data.
    let mut spl_vault_data: SplTokensVaultData = if spl_tokens_vault_data.data_len() == 0 {
        SplTokensVaultData {
            authority: Pubkey::default(),
            total_deposited_amount: Vec::new(),
        }
    } else {
        SplTokensVaultData::try_from_slice(&spl_tokens_vault_data.data.borrow())?
    };
    spl_vault_data.total_deposited_amount.clear();
    spl_vault_data.serialize(&mut *spl_tokens_vault_data.data.borrow_mut())?;

    let mut token_mappings: TokenDecimalMappings = if token_decimal_mappings.data_len() == 0 {
        TokenDecimalMappings {
            authority: *chain_admin.key,
            mappings: Vec::new(),
        }
    } else {
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings.data.borrow())?
    };
    token_mappings.mappings.clear();
    token_mappings.serialize(&mut *token_decimal_mappings.data.borrow_mut())?;
    msg!("Tokens Gateway initialization successful");
    Ok(())
}
