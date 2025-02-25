use crate::core::error::ProgramCustomError;
use crate::core::state::{
    DepositMessagesBuffer, ExecutionMessageBuffer, ForcedWithdrawMessagesBuffer,
    LayerZeroMessagesBuffer,
};
use crate::utils::constants::{
    DEPOSIT_BUFFER_PREFIX, EXECUTION_BUFFER_PREFIX, FORCED_WITHDRAWAL_BUFFER_PREFIX,
    LAYER_ZERO_BUFFER_PREFIX, MAX_QUEUE_SIZE,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn initialize_deposit_buffer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
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

    // deposit_message_buffer
    let (expected_deposit_pda, deposit_bump) =
        Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_deposit_pda != *deposit_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
    if deposit_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(deposit_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            role_manager_acc.key,
            required_lamports,
            deposit_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                role_manager_acc.clone(),
                system_program.clone(),
            ],
            &[&[DEPOSIT_BUFFER_PREFIX.as_bytes(), &[deposit_bump]]],
        )?;
    }

    //forced_withdrawal_buffer
    let (expected_forced_withdrawal_pda, forced_withdraw_bump) =
        Pubkey::find_program_address(&[FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_forced_withdrawal_pda != *forced_withdrawal_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
    if forced_withdrawal_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(forced_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            role_manager_acc.key,
            required_lamports,
            forced_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                role_manager_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                FORCED_WITHDRAWAL_BUFFER_PREFIX.as_bytes(),
                &[forced_withdraw_bump],
            ]],
        )?;
    }

    // layer_zero_buffer
    let (expected_layer_zero_pda, layer_zero_bump) =
        Pubkey::find_program_address(&[LAYER_ZERO_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_layer_zero_pda != *layer_zero_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
    if layer_zero_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(layer_zero_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            role_manager_acc.key,
            required_lamports,
            layer_zero_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                role_manager_acc.clone(),
                system_program.clone(),
            ],
            &[&[LAYER_ZERO_BUFFER_PREFIX.as_bytes(), &[layer_zero_bump]]],
        )?;
    }

    //execution_messages_buffer_acc
    let (expected_execution_pda, execution_message_bump) =
        Pubkey::find_program_address(&[EXECUTION_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_execution_pda != *execution_messages_buffer_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
    if execution_messages_buffer_acc.data_is_empty() {
        let required_lamports = rent.minimum_balance(execution_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            role_manager_acc.key,
            required_lamports,
            execution_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                role_manager_acc.clone(),
                system_program.clone(),
            ],
            &[&[
                EXECUTION_BUFFER_PREFIX.as_bytes(),
                &[execution_message_bump],
            ]],
        )?;
    }

    let mut deposit_messages_buffer =
        DepositMessagesBuffer::try_from_slice(&deposit_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    deposit_messages_buffer.deposit_messages = Vec::new();
    deposit_messages_buffer.deposit_nonce = 0;

    deposit_messages_buffer
        .serialize(&mut *deposit_messages_buffer_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut forced_withdrawal_messages_buffer = ForcedWithdrawMessagesBuffer::try_from_slice(
        &forced_withdrawal_messages_buffer_acc.data.borrow(),
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    forced_withdrawal_messages_buffer.withdraw_messages = Vec::new();
    forced_withdrawal_messages_buffer.withdraw_nonce = 0;

    forced_withdrawal_messages_buffer
        .serialize(&mut *forced_withdrawal_messages_buffer_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut layer_zero_messages_buffer =
        LayerZeroMessagesBuffer::try_from_slice(&layer_zero_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    layer_zero_messages_buffer.lz_messages = Vec::new();
    layer_zero_messages_buffer.lz_nonce = 0;

    layer_zero_messages_buffer
        .serialize(&mut *layer_zero_messages_buffer_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut execution_messages_buffer =
        ExecutionMessageBuffer::try_from_slice(&execution_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    execution_messages_buffer.withdrawals = Vec::new();

    execution_messages_buffer
        .serialize(&mut *execution_messages_buffer_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    Ok(())
}
