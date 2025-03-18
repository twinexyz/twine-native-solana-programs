use crate::core::error::ProgramCustomError;
use crate::core::state::{
    DepositMessagesBuffer, ExecutionMessageBuffer, ForcedWithdrawMessagesBuffer,
    LayerZeroMessagesBuffer,
};
use crate::utils::constants::{
    DEPOSIT_BUFFER_PREFIX, EXECUTION_BUFFER_PREFIX, FORCED_WITHDRAWAL_BUFFER_PREFIX,
    LAYER_ZERO_BUFFER_PREFIX, MAX_QUEUE_SIZE, ROLE_MANAGER_PREFIX,
};
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

pub fn initialize_message_buffer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deposit_messages_buffer_acc = next_account_info(account_iter)?;
    let forced_withdrawal_messages_buffer_acc = next_account_info(account_iter)?;
    let layer_zero_messages_buffer_acc = next_account_info(account_iter)?;
    let execution_messages_buffer_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    let deposit_buffer_space = 8 + 8 + (MAX_QUEUE_SIZE * 200);
    let forced_buffer_space = 8 + 8 + (MAX_QUEUE_SIZE * 200);
    let layer_zero_buffer_space = 8 + 8 + (MAX_QUEUE_SIZE * 200);
    let execution_buffer_space = 8 + 8 + (MAX_QUEUE_SIZE * 200);

    let rent = Rent::get()?;

    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate RoleManager PDA
    let (expected_role_manager_pda, _role_manager_bump_seed) =
        Pubkey::find_program_address(&[ROLE_MANAGER_PREFIX.as_bytes()], program_id);
    if expected_role_manager_pda != *role_manager_acc.key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    /**************************
     * Deposit Message Buffer *
     **************************/

    // Dervive and validate PDA
    let (expected_deposit_pda, deposit_bump) =
        Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_deposit_pda != *deposit_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }

    if deposit_messages_buffer_acc.data_is_empty() {
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

    // Deserialize and update account data
    let mut deposit_buffer_data =
        DepositMessagesBuffer::try_from_slice(&deposit_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if deposit_buffer_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    deposit_buffer_data.is_initialized = true;
    deposit_buffer_data.deposit_messages = Vec::new();
    deposit_buffer_data.deposit_nonce = 0;

    // Serialize account data
    deposit_buffer_data
        .serialize(&mut &mut deposit_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Deposit Message Buffer Initialized");

    /**********************************
     * Forced Withdraw Message Buffer *
     **********************************/

    // Dervive and validate PDA
    let (expected_forced_withdrawal_pda, forced_withdraw_bump) =
        Pubkey::find_program_address(&[FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_forced_withdrawal_pda != *forced_withdrawal_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }

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

    // Deserialize and update account data
    let mut forced_withdraw_buffer_data = ForcedWithdrawMessagesBuffer::try_from_slice(
        &forced_withdrawal_messages_buffer_acc.data.borrow(),
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    if forced_withdraw_buffer_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    forced_withdraw_buffer_data.is_initialized = true;
    forced_withdraw_buffer_data.withdraw_messages = Vec::new();
    forced_withdraw_buffer_data.withdraw_nonce = 0;

    // Serialize account data
    forced_withdraw_buffer_data
        .serialize(&mut &mut forced_withdrawal_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Forced Withdraw Message Buffer Initialized");

    /*****************************
     * Layer Zero Message Buffer *
     *****************************/

    // Dervive and validate PDA
    let (expected_layer_zero_pda, layer_zero_bump) =
        Pubkey::find_program_address(&[LAYER_ZERO_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_layer_zero_pda != *layer_zero_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
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

    // Deserialize and update account data
    let mut layer_zero_buffer_data =
        LayerZeroMessagesBuffer::try_from_slice(&layer_zero_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if layer_zero_buffer_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    layer_zero_buffer_data.is_initialized = true;
    layer_zero_buffer_data.lz_messages = Vec::new();
    layer_zero_buffer_data.lz_nonce = 0;

    // Serialize account data
    layer_zero_buffer_data
        .serialize(&mut &mut layer_zero_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Layer Zero Message Buffer Initialized");

    /****************************
     * Execution Message Buffer *
     ****************************/

    // Dervive and validate PDA
    let (expected_execution_pda, execution_message_bump) =
        Pubkey::find_program_address(&[EXECUTION_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_execution_pda != *execution_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
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
                EXECUTION_BUFFER_PREFIX.as_bytes(),
                &[execution_message_bump],
            ]],
        )?;
    }

    // Deserialize and update account data
    let mut execution_buffer_data =
        ExecutionMessageBuffer::try_from_slice(&execution_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if execution_buffer_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    execution_buffer_data.is_initialized = true;
    execution_buffer_data.withdrawals = Vec::new();

    // Serialize account data
    execution_buffer_data
        .serialize(&mut &mut execution_messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    msg!("Execution Message Buffer Initialized");

    Ok(())
}
