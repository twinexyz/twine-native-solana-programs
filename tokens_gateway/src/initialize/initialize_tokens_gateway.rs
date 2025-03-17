use crate::core::error::ProgramCustomError;
use crate::core::state::{
    ExecutedWithdrawals, NativeTokenVaultData, SplTokensVaultData, TokenDecimalMappings,
    TokenDepositData,
};
use crate::utils::constants::{
    EXECUTED_WITHDRAWALS_PREFIX, MAX_ROLES, MAX_TOKENS, NATIVE_DATA_PREFIX, NATIVE_TOKEN_PREFIX,
    SPL_DATA_PREFIX, TOKEN_DECIMAL_MAPPING_PREFIX,
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

pub fn initialize_tokens_gateway(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    // Expected accounts in order:
    // [0] chain_admin (signer)
    // [1] native_token_vault
    // [2] native_token_vault_data (writable, owner = program_id)
    // [3] spl_tokens_vault_data (writable, owner = program_id)
    // [4] executed_withdrawals_buffer
    // [5] token_decimal_mappings (writable, owner = program_id)
    // [6] role_manager (used for further validations; not used here)
    // [7] Chain Admin (signer)
    // [8] System Program account
    let native_token_vault_acc = next_account_info(account_iter)?;
    let native_token_vault_data_acc = next_account_info(account_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_iter)?;
    let _role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;
    let rent = Rent::get()?;
    // Verify that the chain admin is a signer.
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Create native token vault
    let native_vault_seeds = &[NATIVE_TOKEN_PREFIX.as_bytes()];
    let (native_vault_key, native_vault_bump) =
        Pubkey::find_program_address(native_vault_seeds, program_id);
    if native_vault_key != *native_token_vault_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let lamports = rent.minimum_balance(0);
    if native_token_vault_acc.lamports() < lamports {
        invoke_signed(
            &system_instruction::create_account(
                chain_admin_acc.key,
                native_token_vault_acc.key,
                lamports,
                0,
                &solana_program::system_program::ID,
            ),
            &[
                chain_admin_acc.clone(),
                native_token_vault_acc.clone(),
                system_program.clone(),
            ],
            &[&[NATIVE_TOKEN_PREFIX.as_bytes(), &[native_vault_bump]]],
        )?;
    }

    // Initialize native token vault data
    let native_data_seeds = &[NATIVE_DATA_PREFIX.as_bytes()];
    let (native_data_key, native_data_bump) =
        Pubkey::find_program_address(native_data_seeds, program_id);
    if native_data_key != *native_token_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + std::mem::size_of::<NativeTokenVaultData>();
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin_acc.key,
            native_token_vault_data_acc.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin_acc.clone(),
            native_token_vault_data_acc.clone(),
            system_program.clone(),
        ],
        &[&[NATIVE_DATA_PREFIX.as_bytes(), &[native_data_bump]]],
    )?;

    let spl_data_seeds = &[SPL_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDepositData>());
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin_acc.key,
            spl_tokens_vault_data_acc.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin_acc.clone(),
            spl_tokens_vault_data_acc.clone(),
            system_program.clone(),
        ],
        &[&[SPL_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;

    let executed_withdrawals_seeds = &[EXECUTED_WITHDRAWALS_PREFIX.as_bytes()];
    let (executed_withdrawals_key, executed_withdrawals_bump) =
        Pubkey::find_program_address(executed_withdrawals_seeds, program_id);
    if executed_withdrawals_key != *executed_withdrawals_buffer_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + ExecutedWithdrawals::SPACE;
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin_acc.key,
            executed_withdrawals_buffer_acc.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin_acc.clone(),
            executed_withdrawals_buffer_acc.clone(),
            system_program.clone(),
        ],
        &[&[
            EXECUTED_WITHDRAWALS_PREFIX.as_bytes(),
            &[executed_withdrawals_bump],
        ]],
    )?;

    let mapping_seeds = &[TOKEN_DECIMAL_MAPPING_PREFIX.as_bytes()];
    let (mapping_key, mapping_bump) = Pubkey::find_program_address(mapping_seeds, program_id);
    if mapping_key != *token_decimal_mappings_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let space = 8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDecimalMappings>());
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin_acc.key,
            token_decimal_mappings_acc.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin_acc.clone(),
            token_decimal_mappings_acc.clone(),
            system_program.clone(),
        ],
        &[&[TOKEN_DECIMAL_MAPPING_PREFIX.as_bytes(), &[mapping_bump]]],
    )?;

    let mut native_vault_data =
        NativeTokenVaultData::try_from_slice(&native_token_vault_data_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    native_vault_data.total_deposits = 0;
    native_vault_data
        .serialize(&mut *native_token_vault_data_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut spl_vault_data =
        SplTokensVaultData::try_from_slice(&spl_tokens_vault_data_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    spl_vault_data.total_deposited_amount = Vec::new();
    spl_vault_data
        .serialize(&mut *spl_tokens_vault_data_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut executed_withdrawals_data =
        ExecutedWithdrawals::try_from_slice(&executed_withdrawals_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    executed_withdrawals_data.withdrawal_nonce_lower_bound = 0;
    executed_withdrawals_data.executed_withdrawal_nonces = Vec::new();

    executed_withdrawals_data
        .serialize(&mut *executed_withdrawals_buffer_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut token_mappings_data =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    token_mappings_data.mappings = Vec::new();

    token_mappings_data
        .serialize(&mut *token_decimal_mappings_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Tokens Gateway initialization successful");
    Ok(())
}
