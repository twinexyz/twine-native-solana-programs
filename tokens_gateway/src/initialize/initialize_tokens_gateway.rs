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
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedPayoutsBuffer, ExecutedWithdrawalsBuffer, NativeTokenVaultData,
            SplTokensVaultData, TokenDecimalMappings, TokenDepositData,
        },
    },
    utils::{
        address_derivation::{
            derive_executed_payouts_buffer, derive_executed_withdrawals_buffer,
            derive_gateway_role_manager, derive_native_token_vault, derive_native_token_vault_data,
            derive_spl_tokens_vault_data, derive_token_decimal_mappings,
        },
        constants::{
            EXECUTED_PAYOUTS_BUFFER_PREFIX, EXECUTED_WITHDRAWALS_BUFFER_PREFIX, MAX_TOKENS,
            NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX,
            SPL_TOKENS_VAULT_DATA_PREFIX, TOKEN_DECIMAL_MAPPINGS_PREFIX,
        },
    },
};

pub fn initialize_tokens_gateway(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    // Expected accounts in order:
    // [0] native_token_vault
    // [1] native_token_vault_data (writable, owner = program_id)
    // [2] spl_tokens_vault_data (writable, owner = program_id)
    // [3] executed_payout_buffer_acc (writable, owner = program_id)
    // [4] executed_withdrawals_buffer (writable, owner = program_id)
    // [5] token_decimal_mappings (writable, owner = program_id)
    // [6] role_manager (used for further validations; not used here)
    // [7] chain_admin (signer)
    // [8] system_program
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let executed_payouts_buffer_acc = next_account_info(account_info_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let rent = Rent::default();

    validate_accounts(
        native_token_vault_acc,
        native_token_vault_data_acc,
        spl_tokens_vault_data_acc,
        executed_payouts_buffer_acc,
        executed_withdrawals_buffer_acc,
        token_decimal_mappings_acc,
        role_manager_acc,
        chain_admin_acc,
        system_program,
        program_id,
    )?;

    // Create native token vault
    let (_, native_token_vault_bump) = derive_native_token_vault(&program_id);

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
            &[&[
                NATIVE_TOKEN_VAULT_PREFIX.as_bytes(),
                &[native_token_vault_bump],
            ]],
        )?;
    }

    let (_, native_token_vault_data_bump) = derive_native_token_vault_data(&program_id);

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
        &[&[
            NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
            &[native_token_vault_data_bump],
        ]],
    )?;

    let (_, spl_tokens_vault_data_bump) = derive_spl_tokens_vault_data(&program_id);

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
        &[&[
            SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes(),
            &[spl_tokens_vault_data_bump],
        ]],
    )?;

    /*******************************
     * Executed withdrawals Buffer *
     *******************************/

    let (_, executed_withdrawals_buffer_bump) = derive_executed_withdrawals_buffer(&program_id);

    let space = 8 + ExecutedWithdrawalsBuffer::SPACE;
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
            EXECUTED_WITHDRAWALS_BUFFER_PREFIX.as_bytes(),
            &[executed_withdrawals_buffer_bump],
        ]],
    )?;
    /****************************
     * Executed Payouts Buffer  *
     ****************************/
    let (_, executed_withdrawals_buffer_bump) = derive_executed_payouts_buffer(&program_id);

    let space = 8 + ExecutedPayoutsBuffer::SPACE;
    let lamports = rent.minimum_balance(space);
    invoke_signed(
        &system_instruction::create_account(
            chain_admin_acc.key,
            executed_payouts_buffer_acc.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            chain_admin_acc.clone(),
            executed_payouts_buffer_acc.clone(),
            system_program.clone(),
        ],
        &[&[
            EXECUTED_PAYOUTS_BUFFER_PREFIX.as_bytes(),
            &[executed_withdrawals_buffer_bump],
        ]],
    )?;
    /****************************
     * Token Decimal Mapping *
     ****************************/

    let (_, token_decimal_mappings_bump) = derive_token_decimal_mappings(&program_id);

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
        &[&[
            TOKEN_DECIMAL_MAPPINGS_PREFIX.as_bytes(),
            &[token_decimal_mappings_bump],
        ]],
    )?;

    let native_token_vault_data = NativeTokenVaultData {
        is_initialized: true,
        total_deposits: 0,
    };
    let mut native_vault_data_data = native_token_vault_data_acc.data.borrow_mut();
    native_token_vault_data
        .serialize(&mut &mut native_vault_data_data[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    let spl_tokens_vault_data = SplTokensVaultData {
        is_initialized: true,
        total_deposited_amount: Vec::new(),
    };
    let mut spl_tokens_vault_data_data = spl_tokens_vault_data_acc.data.borrow_mut();
    spl_tokens_vault_data
        .serialize(&mut &mut spl_tokens_vault_data_data[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let executed_withdrawals_buffer = ExecutedWithdrawalsBuffer {
        is_initialized: true,
        withdrawal_nonce_lower_bound: 0,
        executed_withdrawal_nonces: Vec::new(),
    };

    let mut executed_withdrawals_buffer_data = executed_withdrawals_buffer_acc.data.borrow_mut();
    executed_withdrawals_buffer
        .serialize(&mut &mut executed_withdrawals_buffer_data[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let executed_payouts_buffer = ExecutedPayoutsBuffer {
        is_initialized: true,
        payout_nonce_lower_bound: 0,
        executed_payout_nonces: Vec::new(),
    };
    let mut executed_payouts_buffer_data = executed_payouts_buffer_acc.data.borrow_mut();
    executed_withdrawals_buffer
        .serialize(&mut &mut executed_payouts_buffer_data[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let token_decimal_mappings_data = TokenDecimalMappings {
        is_initialized: true,
        mappings: Vec::new(),
    };
    let mut token_decimal_mappings_data_data = token_decimal_mappings_acc.data.borrow_mut();

    token_decimal_mappings_data
        .serialize(&mut &mut token_decimal_mappings_data_data[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!(
    "EVENT:TokensGatewayInitialized: native_token_vault={}, native_token_vault_data={}, spl_tokens_vault_data={}, executed_payouts_buffer={}, executed_withdrawals_buffer={},token_decimal_mappings={}, chain_admin={}",
    native_token_vault_acc.key,
    native_token_vault_data_acc.key,
    spl_tokens_vault_data_acc.key,
    executed_payouts_buffer_acc.key,
    executed_withdrawals_buffer_acc.key,
    token_decimal_mappings_acc.key,
    chain_admin_acc.key
);

    Ok(())
}

fn validate_accounts(
    native_token_vault_acc: &AccountInfo,
    native_token_vault_data_acc: &AccountInfo,
    spl_tokens_vault_data_acc: &AccountInfo,
    executed_payouts_buffer_acc: &AccountInfo,
    executed_withdrawals_buffer_acc: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
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

    let (native_token_vault_key, _) = derive_native_token_vault(&program_id);
    if native_token_vault_key != *native_token_vault_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (native_token_vault_data_key, _) = derive_native_token_vault_data(&program_id);
    if native_token_vault_data_key != *native_token_vault_data_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (spl_tokens_vault_data_key, _) = derive_spl_tokens_vault_data(&program_id);
    if spl_tokens_vault_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (executed_withdrawals_buffer_key, _) = derive_executed_withdrawals_buffer(&program_id);
    if executed_withdrawals_buffer_key != *executed_withdrawals_buffer_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (executed_payouts_buffer_key, _) = derive_executed_payouts_buffer(&program_id);
    if executed_payouts_buffer_key != *executed_payouts_buffer_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (token_decimal_mappings_key, _) = derive_token_decimal_mappings(&program_id);
    if token_decimal_mappings_key != *token_decimal_mappings_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }
    let (role_manager_key, _) = derive_gateway_role_manager(&program_id);
    if role_manager_key != *role_manager_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let data = native_token_vault_data_acc.data.borrow();
    if !data.is_empty() {
        let mut data_slice = &data[..];
        if let Ok(native_token_vault_data) = NativeTokenVaultData::deserialize(&mut data_slice) {
            if native_token_vault_data.is_initialized {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        }
    }
    let data = spl_tokens_vault_data_acc.data.borrow();
    if !data.is_empty() {
        let mut data_slice = &data[..];
        if let Ok(spl_tokens_vault_data) = SplTokensVaultData::deserialize(&mut data_slice) {
            if spl_tokens_vault_data.is_initialized {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        }
    }

    let data = executed_withdrawals_buffer_acc.data.borrow();
    if !data.is_empty() {
        let mut data_slice = &data[..];
        if let Ok(executed_withdrawals_buffer_data) =
            ExecutedWithdrawalsBuffer::deserialize(&mut data_slice)
        {
            if executed_withdrawals_buffer_data.is_initialized {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        }
    }
    let data = executed_payouts_buffer_acc.data.borrow();
    if !data.is_empty() {
        let mut data_slice = &data[..];
        if let Ok(executed_payouts_buffer_data) =
            ExecutedPayoutsBuffer::deserialize(&mut data_slice)
        {
            if executed_payouts_buffer_data.is_initialized {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        }
    }

    let data = token_decimal_mappings_acc.data.borrow();
    if !data.is_empty() {
        let mut data_slice = &data[..];
        if let Ok(token_decimal_mappings_data) = TokenDecimalMappings::deserialize(&mut data_slice)
        {
            if token_decimal_mappings_data.is_initialized {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        }
    }

    Ok(())
}