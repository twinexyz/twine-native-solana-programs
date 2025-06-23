use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedWithdrawalsBuffer, NativeTokenVaultData, SplTokensVaultData,
            TokenDecimalMappings, TokenDepositData,
        },
    },
    utils::{
        address_derivation::{
            derive_executed_withdrawals_buffer, derive_gateway_role_manager,
            derive_native_token_vault, derive_native_token_vault_data,
            derive_spl_tokens_vault_data, derive_token_decimal_mappings,
        },
        constants::{
            EXECUTED_WITHDRAWALS_BUFFER_PREFIX, MAX_TOKENS, NATIVE_TOKEN_VAULT_DATA_PREFIX,
            NATIVE_TOKEN_VAULT_PREFIX, SPL_TOKENS_VAULT_DATA_PREFIX, TOKEN_DECIMAL_MAPPINGS_PREFIX,
        },
        
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::program::invoke_signed;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

pub fn initialize_tokens_gateway(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    // Expected accounts in order:
    // [0] native_token_vault
    // [1] native_token_vault_data (writable, owner = program_id)
    // [2] spl_tokens_vault_data (writable, owner = program_id)
    // [3] executed_withdrawals_buffer (writable, owner = program_id)
    // [4] token_decimal_mappings (writable, owner = program_id)
    // [5] role_manager (used for further validations; not used here)
    // [6] chain_admin (signer)
    // [7] system_program
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
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

    let token_decimal_mappings_data = TokenDecimalMappings {
        is_initialized: true,
        mappings: Vec::new(),
    };
    let mut token_decimal_mappings_data_data = token_decimal_mappings_acc.data.borrow_mut();

    token_decimal_mappings_data
        .serialize(&mut &mut token_decimal_mappings_data_data[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!(
    "EVENT:TokensGatewayInitialized: native_token_vault={}, native_token_vault_data={}, spl_tokens_vault_data={}, executed_withdrawals_buffer={}, token_decimal_mappings={}, chain_admin={}",
    native_token_vault_acc.key,
    native_token_vault_data_acc.key,
    spl_tokens_vault_data_acc.key,
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

#[cfg(test)]
fn invoke_signed(
    _instruction: &solana_program::instruction::Instruction,
    account_infos: &[solana_program::account_info::AccountInfo],
    _signer_seeds: &[&[&[u8]]],
) -> solana_program::entrypoint::ProgramResult {
    // Clear account data for mock initialization
    for account_info in account_infos.iter() {
        if account_info.is_writable {
            let mut data = account_info.data.borrow_mut();
            data.fill(0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::constants::INITIAL_CHAIN_ADMIN;
    use solana_program::{
        account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, rent::Rent, system_program,
    };

    use std::str::FromStr;

    fn create_test_account_info<'a>(
        key: &'a Pubkey,
        is_signer: bool,
        is_writable: bool,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        owner: &'a mut Pubkey,
    ) -> AccountInfo<'a> {
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            false,
            Epoch::default(),
        )
    }

    #[test]
    fn test_initialize_tokens_gateway_success() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get all the PDA keys
        let (native_token_vault_key, _) = derive_native_token_vault(&program_id);
        let (native_token_vault_data_key, _) = derive_native_token_vault_data(&program_id);
        let (spl_tokens_vault_data_key, _) = derive_spl_tokens_vault_data(&program_id);
        let (executed_withdrawals_buffer_key, _) = derive_executed_withdrawals_buffer(&program_id);
        let (token_decimal_mappings_key, _) = derive_token_decimal_mappings(&program_id);
        let (role_manager_key, _) = derive_gateway_role_manager(&program_id);
        let chain_admin_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        let rent = Rent::default();

        // Setup account lamports
        let mut native_token_vault_lamports = rent.minimum_balance(0);
        let mut native_token_vault_data_lamports =
            rent.minimum_balance(8 + std::mem::size_of::<NativeTokenVaultData>());
        let mut spl_tokens_vault_data_lamports = rent
            .minimum_balance(8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDepositData>()));
        let mut executed_withdrawals_buffer_lamports =
            rent.minimum_balance(8 + ExecutedWithdrawalsBuffer::SPACE);
        let mut token_decimal_mappings_lamports = rent.minimum_balance(
            8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDecimalMappings>()),
        );
        let mut role_manager_lamports = 1_000_000;
        let mut chain_admin_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup account data with proper sizes
        let mut native_token_vault_data = vec![0u8; 0];
        let mut native_token_vault_data_data =
            vec![0u8; 8 + std::mem::size_of::<NativeTokenVaultData>()];
        let mut spl_tokens_vault_data_data =
            vec![0u8; 8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDepositData>())];
        let mut executed_withdrawals_buffer_data = vec![0u8; 8 + ExecutedWithdrawalsBuffer::SPACE];
        let mut token_decimal_mappings_data =
            vec![0u8; 8 + 32 + 4 + (MAX_TOKENS * std::mem::size_of::<TokenDecimalMappings>())];
        let mut role_manager_data = vec![0u8; 1000];
        let mut chain_admin_data = vec![];
        let mut system_program_data = vec![];

        // Setup owners
        let mut native_token_vault_owner = system_program_id;
        let mut native_token_vault_data_owner = program_id;
        let mut spl_tokens_vault_data_owner = program_id;
        let mut executed_withdrawals_buffer_owner = program_id;
        let mut token_decimal_mappings_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut chain_admin_owner = system_program_id;
        let mut system_program_owner = system_program_id;

        // Create account infos in the correct order
        let native_token_vault_account = create_test_account_info(
            &native_token_vault_key,
            false,
            true,
            &mut native_token_vault_lamports,
            &mut native_token_vault_data,
            &mut native_token_vault_owner,
        );

        let native_token_vault_data_account = create_test_account_info(
            &native_token_vault_data_key,
            false,
            true,
            &mut native_token_vault_data_lamports,
            &mut native_token_vault_data_data,
            &mut native_token_vault_data_owner,
        );

        let spl_tokens_vault_data_account = create_test_account_info(
            &spl_tokens_vault_data_key,
            false,
            true,
            &mut spl_tokens_vault_data_lamports,
            &mut spl_tokens_vault_data_data,
            &mut spl_tokens_vault_data_owner,
        );

        let executed_withdrawals_buffer_account = create_test_account_info(
            &executed_withdrawals_buffer_key,
            false,
            true,
            &mut executed_withdrawals_buffer_lamports,
            &mut executed_withdrawals_buffer_data,
            &mut executed_withdrawals_buffer_owner,
        );

        let token_decimal_mappings_account = create_test_account_info(
            &token_decimal_mappings_key,
            false,
            true,
            &mut token_decimal_mappings_lamports,
            &mut token_decimal_mappings_data,
            &mut token_decimal_mappings_owner,
        );

        let role_manager_account = create_test_account_info(
            &role_manager_key,
            false,
            false,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        let chain_admin_account = create_test_account_info(
            &chain_admin_key,
            true, // Must be signer
            false,
            &mut chain_admin_lamports,
            &mut chain_admin_data,
            &mut chain_admin_owner,
        );

        let system_program_account = create_test_account_info(
            &system_program_id,
            false,
            false,
            &mut system_program_lamports,
            &mut system_program_data,
            &mut system_program_owner,
        );

        // Create accounts array in the CORRECT order matching the function
        let accounts = vec![
            native_token_vault_account.clone(),
            native_token_vault_data_account.clone(),
            spl_tokens_vault_data_account.clone(),
            executed_withdrawals_buffer_account.clone(),
            token_decimal_mappings_account.clone(),
            role_manager_account,
            chain_admin_account,
            system_program_account,
        ];

        // Call the initialization function
        let result = initialize_tokens_gateway(&program_id, &accounts);
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        // Verify native token vault data initialization
        let native_vault_data_ref = native_token_vault_data_account.data.borrow();
        let mut native_vault_data_slice = &native_vault_data_ref[..];
        let deserialized_native_vault =
            NativeTokenVaultData::deserialize(&mut native_vault_data_slice)?;
        assert!(
            deserialized_native_vault.is_initialized,
            "Native token vault should be initialized"
        );
        assert_eq!(
            deserialized_native_vault.total_deposits, 0,
            "Total deposits should be 0"
        );

        // Verify SPL tokens vault data initialization
        let spl_vault_data_ref = spl_tokens_vault_data_account.data.borrow();
        let mut spl_vault_data_slice = &spl_vault_data_ref[..];
        let deserialized_spl_vault = SplTokensVaultData::deserialize(&mut spl_vault_data_slice)?;
        assert!(
            deserialized_spl_vault.is_initialized,
            "SPL tokens vault should be initialized"
        );
        assert!(
            deserialized_spl_vault.total_deposited_amount.is_empty(),
            "Total deposited amount should be empty"
        );

        // Verify executed withdrawals buffer initialization
        let executed_withdrawals_data_ref = executed_withdrawals_buffer_account.data.borrow();
        let mut executed_withdrawals_data_slice = &executed_withdrawals_data_ref[..];
        let deserialized_executed_withdrawals =
            ExecutedWithdrawalsBuffer::deserialize(&mut executed_withdrawals_data_slice)?;
        assert!(
            deserialized_executed_withdrawals.is_initialized,
            "Executed withdrawals buffer should be initialized"
        );
        assert_eq!(
            deserialized_executed_withdrawals.withdrawal_nonce_lower_bound, 0,
            "Withdrawal nonce lower bound should be 0"
        );
        assert!(
            deserialized_executed_withdrawals
                .executed_withdrawal_nonces
                .is_empty(),
            "Executed withdrawal nonces should be empty"
        );

        // Verify token decimal mappings initialization
        let token_decimal_data_ref = token_decimal_mappings_account.data.borrow();
        let mut token_decimal_data_slice = &token_decimal_data_ref[..];
        let deserialized_token_decimal =
            TokenDecimalMappings::deserialize(&mut token_decimal_data_slice)?;
        assert!(
            deserialized_token_decimal.is_initialized,
            "Token decimal mappings should be initialized"
        );
        assert!(
            deserialized_token_decimal.mappings.is_empty(),
            "Token decimal mappings should be empty"
        );

        Ok(())
    }
}
