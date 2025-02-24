use crate::core::state::{NativeTokenVaultData, TokenDecimalMappings, TokenDepositData};
use crate::utils::constants::{
    MAX_ROLES, MAX_TOKENS, NATIVE_DATA_PREFIX, NATIVE_TOKEN_PREFIX, ROLE_MANAGER_PREFIX,
    SPL_DATA_PREFIX, TOKEN_DECIMAL_MAPPING_PREFIX,
};
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
pub fn initialize_pdas(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    // Expected accounts order:
    // [0] Role Manager PDA account (writable)
    // [1] Native Token Vault PDA account (writable)
    // [2] Native Token Vault Data PDA account (writable)
    // [3] SPL Tokens Vault Data PDA account (writable)
    // [4] Token Decimal Mappings PDA account (writable)
    // [5] Chain Admin (signer)
    // [6] System Program account
    let account_info_iter = &mut accounts.iter();
    let role_manager = next_account_info(account_info_iter)?;
    let native_token_vault = next_account_info(account_info_iter)?;
    let native_token_vault_data = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data = next_account_info(account_info_iter)?;
    let token_decimal_mappings = next_account_info(account_info_iter)?;
    let chain_admin = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let rent = Rent::get()?;

    // --- Role Manager PDA ---
    let role_manager_seeds = &[ROLE_MANAGER_PREFIX.as_bytes()];
    let (expected_role_manager_key, role_bump) =
        Pubkey::find_program_address(role_manager_seeds, program_id);
    if expected_role_manager_key != *role_manager.key {
        msg!("Role Manager PDA key mismatch");
        return Err(ProgramError::InvalidAccountData.into());
    }

    let role_manager_space = 8 + 32 + 4 + (MAX_ROLES * 33);
    if role_manager.data_is_empty() {
        let required_lamports = rent.minimum_balance(role_manager_space);
        let create_ix = system_instruction::create_account(
            chain_admin.key,
            role_manager.key,
            required_lamports,
            role_manager_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin.clone(),
                role_manager.clone(),
                system_program.clone(),
            ],
            &[&[ROLE_MANAGER_PREFIX.as_bytes(), &[role_bump]]],
        )?;
    }
    // Create native token vault (PDA)
    let native_vault_seeds = &[NATIVE_TOKEN_PREFIX.as_bytes()];
    let (native_vault_key, native_vault_bump) =
        Pubkey::find_program_address(native_vault_seeds, program_id);
    if native_vault_key != *native_token_vault.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let lamports = rent.minimum_balance(0);
    if native_token_vault.lamports() < lamports {
        invoke_signed(
            &system_instruction::create_account(
                chain_admin.key,
                native_token_vault.key,
                lamports,
                0,
                &solana_program::system_program::ID,
            ),
            &[
                chain_admin.clone(),
                native_token_vault.clone(),
                system_program.clone(),
            ],
            &[&[NATIVE_TOKEN_PREFIX.as_bytes(), &[native_vault_bump]]],
        )?;
    }
    // Initialize native token vault data
    let native_data_seeds = &[NATIVE_DATA_PREFIX.as_bytes()];
    let (native_data_key, native_data_bump) =
        Pubkey::find_program_address(native_data_seeds, program_id);
    if native_data_key != *native_token_vault_data.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + std::mem::size_of::<NativeTokenVaultData>();
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin.key,
            native_token_vault_data.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin.clone(),
            native_token_vault_data.clone(),
            system_program.clone(),
        ],
        &[&[NATIVE_DATA_PREFIX.as_bytes(), &[native_data_bump]]],
    )?;

    // Initialize SPL tokens vault data
    let spl_data_seeds = &[SPL_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDepositData>());
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin.key,
            spl_tokens_vault_data.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin.clone(),
            spl_tokens_vault_data.clone(),
            system_program.clone(),
        ],
        &[&[SPL_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;

    // Initialize token decimal mappings
    let mapping_seeds = &[TOKEN_DECIMAL_MAPPING_PREFIX.as_bytes()];
    let (mapping_key, mapping_bump) = Pubkey::find_program_address(mapping_seeds, program_id);
    if mapping_key != *token_decimal_mappings.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDecimalMappings>());
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin.key,
            token_decimal_mappings.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin.clone(),
            token_decimal_mappings.clone(),
            system_program.clone(),
        ],
        &[&[TOKEN_DECIMAL_MAPPING_PREFIX.as_bytes(), &[mapping_bump]]],
    )?;

    Ok(())
}
