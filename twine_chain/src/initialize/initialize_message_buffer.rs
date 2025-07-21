use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::program::invoke_signed;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            DepositMessagesBuffer, ExecutionMessageBuffer, ForcedWithdrawMessagesBuffer,
            LayerZeroMessagesBuffer, TwineChainRoleManager,
        },
    },
    utils::{
        address_derivation::{
            derive_deposit_message_buffer, derive_execution_message_buffer,
            derive_forced_withdraw_message_buffer, derive_layer_zero_message_buffer,
            derive_role_manager, verify_derived_address, verify_owner, verify_system_program,
        },
        constants::{
            CHAIN_ID,DEPOSIT_BUFFER_PREFIX, EXECUTION_MESSAGE_BUFFER_PREFIX,
            FORCED_WITHDRAWAL_BUFFER_PREFIX, LAYER_ZERO_BUFFER_PREFIX,
        },
    },
};

/// Initializes a new on-chain message buffer account.
///
///
/// # Parameters
/// - `program_id`: The public key of the current program, used for PDA checks.
/// - `accounts`: A list of accounts expected in the following order:
///
///     0. `[writable]` Deposit messages buffer account (PDA-owned)
///         - Used to store messages related to deposit events.
///
///     1. `[writable]` Forced withdrawal messages buffer account (PDA-owned)
///         - Used to track forced withdrawal requests.
///
///     2. `[writable]` LayerZero messages buffer account (PDA-owned)
///         - Stores incoming messages from LayerZero protocol integration.
///
///     3. `[writable]` Execution messages buffer account (PDA-owned)
///         - Hold withdrawal messages that are ready for execution.
///
///     4. `[]` Role manager account
///         - Used to check that the caller has sufficient privileges.
///
///     5. `[signer]` Chain admin account
///         - The admin invoking the initialization; must be authorized via the role manager.
///
///     6. `[]` System program
///         - Required for allocating and assigning accounts on Solana.
///

pub fn initialize_message_buffer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deposit_messages_buffer_acc = next_account_info(account_iter)?;
    let forced_withdrawal_messages_buffer_acc = next_account_info(account_iter)?;
    let layer_zero_messages_buffer_acc = next_account_info(account_iter)?;
    let execution_messages_buffer_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Validate Provided accounts
    let (deposit_bump, forced_withdraw_bump, layer_zero_bump, execution_message_bump) =
        validate_accounts(
            program_id,
            deposit_messages_buffer_acc,
            forced_withdrawal_messages_buffer_acc,
            layer_zero_messages_buffer_acc,
            execution_messages_buffer_acc,
            role_manager_acc,
            chain_admin_acc,
            system_program,
        )?;
    let rent = Rent::default();

    /**************************
     * Deposit Message Buffer *
     *************************/
    if deposit_messages_buffer_acc.data_is_empty() {
        let deposit_buffer_space = 10240;
        let required_lamports = rent.minimum_balance(deposit_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            deposit_messages_buffer_acc.key,
            required_lamports,
            deposit_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                deposit_messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[DEPOSIT_BUFFER_PREFIX.as_bytes(), &[deposit_bump]]],
        )?;
    }

    // Update account data
    let deposit_buffer_data = DepositMessagesBuffer {
        is_initialized: true,
        deposit_nonce: 0,
        chain_id: CHAIN_ID,
        deposit_messages: Vec::new(),
    };

    // Serialize account data
    deposit_buffer_data
        .serialize(&mut &mut deposit_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Deposit Message Buffer Initialized");

    /**********************************
     * Forced Withdraw Message Buffer *
     **********************************/
    let forced_buffer_space = 10240;

    if forced_withdrawal_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(forced_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            forced_withdrawal_messages_buffer_acc.key,
            required_lamports,
            forced_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                forced_withdrawal_messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes(),
                &[forced_withdraw_bump],
            ]],
        )?;
    }

    // Update account data
    let forced_withdraw_buffer_data = ForcedWithdrawMessagesBuffer {
        is_initialized: true,
        withdraw_nonce: 0,
        withdraw_messages: Vec::new(),
    };

    // Serialize account data
    forced_withdraw_buffer_data
        .serialize(&mut &mut forced_withdrawal_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Forced Withdraw Message Buffer Initialized");

    /*****************************
     * Layer Zero Message Buffer *
     *****************************/
    let layer_zero_buffer_space = 10240;

    // Dervive and validate PDA
    if layer_zero_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(layer_zero_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            layer_zero_messages_buffer_acc.key,
            required_lamports,
            layer_zero_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                layer_zero_messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[LAYER_ZERO_BUFFER_PREFIX.as_bytes(), &[layer_zero_bump]]],
        )?;
    }

    // Update account data
    let layer_zero_buffer_data = LayerZeroMessagesBuffer {
        is_initialized: true,
        lz_nonce: 0,
        lz_messages: Vec::new(),
    };

    layer_zero_buffer_data
        .serialize(&mut &mut layer_zero_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Layer Zero Message Buffer Initialized");

    /****************************
     * Execution Message Buffer *
     ****************************/
    let execution_buffer_space = 10240;

    if execution_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(execution_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            execution_messages_buffer_acc.key,
            required_lamports,
            execution_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                execution_messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                EXECUTION_MESSAGE_BUFFER_PREFIX.as_bytes(),
                &[execution_message_bump],
            ]],
        )?;
    }

    // Update account data
    let execution_buffer_data = ExecutionMessageBuffer {
        is_initialized: true,
        withdrawals: Vec::new(),
    };

    // Serialize account data
    execution_buffer_data
        .serialize(&mut &mut execution_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Execution Message Buffer Initialized");

    Ok(())
}

// TODO: Check if chain_admin_acc has required role(Chain Admin)
fn validate_accounts(
    program_id: &Pubkey,
    deposit_messages_buffer_acc: &AccountInfo,
    forced_withdrawal_messages_buffer_acc: &AccountInfo,
    layer_zero_messages_buffer_acc: &AccountInfo,
    execution_messages_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<(u8, u8, u8, u8), ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate Account key and owner
    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;
    verify_owner(role_manager_acc, program_id)?;

    let (expected_deposit_pda, deposit_bump) = derive_deposit_message_buffer(program_id);
    verify_derived_address(expected_deposit_pda, deposit_messages_buffer_acc)?;

    let (expected_forced_withdrawal_pda, forced_withdraw_bump) =
        derive_forced_withdraw_message_buffer(program_id);
    verify_derived_address(
        expected_forced_withdrawal_pda,
        forced_withdrawal_messages_buffer_acc,
    )?;

    let (expected_layer_zero_pda, layer_zero_bump) = derive_layer_zero_message_buffer(program_id);
    verify_derived_address(expected_layer_zero_pda, layer_zero_messages_buffer_acc)?;

    let (expected_execution_pda, execution_message_bump) =
        derive_execution_message_buffer(program_id);
    verify_derived_address(expected_execution_pda, execution_messages_buffer_acc)?;

    verify_system_program(system_program)?;

    // re-initialization guard for deposit buffer
    if !deposit_messages_buffer_acc.data_is_empty() {
        let deposit_buffer_data =
            DepositMessagesBuffer::deserialize(&mut &deposit_messages_buffer_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;
        if deposit_buffer_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // re-initialization guard for forced withdraw buffer
    if !forced_withdrawal_messages_buffer_acc.data_is_empty() {
        let forced_withdraw_buffer_data = ForcedWithdrawMessagesBuffer::deserialize(
            &mut &forced_withdrawal_messages_buffer_acc.data.borrow()[..],
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;

        if forced_withdraw_buffer_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // re-initialization guard for layer zero buffer
    if !layer_zero_messages_buffer_acc.data_is_empty() {
        let layer_zero_buffer_data = LayerZeroMessagesBuffer::deserialize(
            &mut &layer_zero_messages_buffer_acc.data.borrow()[..],
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;

        if layer_zero_buffer_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // re-initialization guard for execution buffer
    if !execution_messages_buffer_acc.data_is_empty() {
        let execution_buffer_data = ExecutionMessageBuffer::deserialize(
            &mut &execution_messages_buffer_acc.data.borrow()[..],
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;

        if execution_buffer_data.is_initialized() {
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

    Ok((
        deposit_bump,
        forced_withdraw_bump,
        layer_zero_bump,
        execution_message_bump,
    ))
}

// Just for testing purpose! This function just allocate sufficient space to PDAs
#[cfg(test)]
fn invoke_signed(
    _ix: &solana_program::instruction::Instruction,
    account_infos: &[solana_program::account_info::AccountInfo],
    _signer_seeds: &[&[&[u8]]],
) -> solana_program::entrypoint::ProgramResult {
    use std::mem;

    for acc in account_infos.iter() {
        if !acc.is_writable {
            continue;
        }
        // For testing purpose, allocate a large space to every PDA to allow serialization
        let space = 10240;
        let leaked: &'static mut [u8] = Box::leak(vec![0u8; space].into_boxed_slice());
        unsafe {
            let mut data_ref = acc.data.borrow_mut();
            *data_ref = mem::transmute::<&'static mut [u8], &mut [u8]>(leaked);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_ROLES};
    use borsh::BorshDeserialize;
    use solana_program::{clock::Epoch, system_program};
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
    fn test_message_buffer_initialization() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (deposit_message_buffer_key, _) = derive_deposit_message_buffer(&program_id);
        let (forced_withdraw_message_buffer_key, _) =
            derive_forced_withdraw_message_buffer(&program_id);
        let (layer_zero_message_buffer_key, _) = derive_layer_zero_message_buffer(&program_id);
        let (execution_message_buffer_key, _) = derive_execution_message_buffer(&program_id);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let chain_admin_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let deposit_message_buffer_space = 10240;
        let forced_withdraw_message_buffer_space = 10240;
        let layer_zero_message_buffer_space = 10240;
        let execution_message_buffer_space = 10240;
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();

        let mut deposit_message_buffer_lamports =
            rent.minimum_balance(deposit_message_buffer_space);
        let mut forced_withdraw_message_buffer_lamports =
            rent.minimum_balance(forced_withdraw_message_buffer_space);
        let mut layer_zero_message_buffer_lamports =
            rent.minimum_balance(layer_zero_message_buffer_space);
        let mut execution_message_buffer_lamports =
            rent.minimum_balance(execution_message_buffer_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut chain_admin_lamports = 1_000_000_000;
        let mut system_program_lamports = 0;

        // Setup account data with proper sizes
        let mut deposit_message_buffer_data = vec![];
        let mut forced_withdraw_message_buffer_data = vec![];
        let mut layer_zero_message_buffer_data = vec![];
        let mut execution_message_buffer_data = vec![];
        let mut chain_admin_data = vec![];
        let mut system_program_data = vec![];

        // Set InitialChainAdmin as chain admin
        let role_manager_dummy_data = TwineChainRoleManager {
            is_initialized: true,
            chain_admin: Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
            twine_operator: Pubkey::default(),
            token_gateway_program: Pubkey::default(),
            roles: vec![],
        };
        let mut role_manager_data = vec![];
        role_manager_dummy_data.serialize(&mut role_manager_data)?;

        // Setup owners
        let mut deposit_message_buffer_owner = program_id;
        let mut forced_withdraw_message_buffer_owner = program_id;
        let mut layer_zero_message_buffer_owner = program_id;
        let mut execution_message_buffer_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut chain_admin_owner = system_program_id;
        let mut system_program_owner = system_program_id;

        // Create required account infos
        let deposit_message_buffer_account = create_test_account_info(
            &deposit_message_buffer_key,
            false,
            true,
            &mut deposit_message_buffer_lamports,
            &mut deposit_message_buffer_data,
            &mut deposit_message_buffer_owner,
        );

        let forced_withdraw_message_buffer_account = create_test_account_info(
            &forced_withdraw_message_buffer_key,
            false,
            true,
            &mut forced_withdraw_message_buffer_lamports,
            &mut forced_withdraw_message_buffer_data,
            &mut forced_withdraw_message_buffer_owner,
        );

        let layer_zero_message_buffer_account = create_test_account_info(
            &layer_zero_message_buffer_key,
            false,
            true,
            &mut layer_zero_message_buffer_lamports,
            &mut layer_zero_message_buffer_data,
            &mut layer_zero_message_buffer_owner,
        );

        let execution_message_buffer_account = create_test_account_info(
            &execution_message_buffer_key,
            false,
            true,
            &mut execution_message_buffer_lamports,
            &mut execution_message_buffer_data,
            &mut execution_message_buffer_owner,
        );

        let role_manager_account = create_test_account_info(
            &role_manager_key,
            false,
            true,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        let chain_admin_account = create_test_account_info(
            &chain_admin_key,
            true,
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

        // Create accounts array in the correct order matching the function
        let accounts = vec![
            deposit_message_buffer_account.clone(),
            forced_withdraw_message_buffer_account.clone(),
            layer_zero_message_buffer_account.clone(),
            execution_message_buffer_account.clone(),
            role_manager_account.clone(),
            chain_admin_account.clone(),
            system_program_account.clone(),
        ];

        // Call the initialize function
        let result = initialize_message_buffer(&program_id, &accounts);
        assert!(result.is_ok(), "Initialization failed: {:?}", result.err());

        // verify deposit message buffer initialization
        let deposit_buffer_data = DepositMessagesBuffer::deserialize(
            &mut &deposit_message_buffer_account.data.borrow()[..],
        )?;

        assert!(
            deposit_buffer_data.is_initialized,
            "Deposit buffer should be initialized"
        );

        assert_eq!(
            deposit_buffer_data.deposit_nonce, 0,
            "Deposit nonce should be 0"
        );

        assert_eq!(
            deposit_buffer_data.deposit_messages.len(),
            0,
            "The should be no deposits"
        );

        // verify forced withdraw message buffer initialization
        let forced_withdraw_buffer_data = ForcedWithdrawMessagesBuffer::deserialize(
            &mut &forced_withdraw_message_buffer_account.data.borrow()[..],
        )?;

        assert!(
            forced_withdraw_buffer_data.is_initialized,
            "Forced Withdraw buffer should be initialized"
        );

        assert_eq!(
            forced_withdraw_buffer_data.withdraw_nonce, 0,
            "Deposit nonce should be 0"
        );

        assert_eq!(
            forced_withdraw_buffer_data.withdraw_messages.len(),
            0,
            "The should be no withdraws"
        );

        // Verify layer zero message buffer initialization
        let layer_zero_buffer_data = LayerZeroMessagesBuffer::deserialize(
            &mut &layer_zero_message_buffer_account.data.borrow()[..],
        )?;

        assert!(
            layer_zero_buffer_data.is_initialized,
            "Layer zero buffer should be initialized"
        );

        assert_eq!(layer_zero_buffer_data.lz_nonce, 0, "Lz nonce should be 0");
        assert_eq!(
            layer_zero_buffer_data.lz_messages.len(),
            0,
            "The should be no layerzero messages"
        );

        let execution_buffer_data = ExecutionMessageBuffer::deserialize(
            &mut &execution_message_buffer_account.data.borrow()[..],
        )?;

        assert!(
            execution_buffer_data.is_initialized,
            "Execution buffer should be initialized"
        );

        assert_eq!(
            execution_buffer_data.withdrawals.len(),
            0,
            "The should be no withdrawals"
        );

        Ok(())
    }
}
