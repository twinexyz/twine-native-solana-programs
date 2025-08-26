use borsh::{BorshDeserialize, BorshSerialize};
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
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutionMessageBuffer, LayerZeroMessagesBuffer, MessagesBuffer, TwineChainRoleManager
        },
    },
    utils::{
        address_derivation::{
            derive_execution_message_buffer,derive_layer_zero_message_buffer, derive_messages_buffer, derive_role_manager, verify_derived_address, verify_owner, verify_system_program
        },
        constants::{
            CHAIN_ID,EXECUTION_MESSAGE_BUFFER_PREFIX,
             LAYER_ZERO_BUFFER_PREFIX, MESSAGES_BUFFER_PREFIX,
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
///     0. `[writable]` messages buffer account (PDA-owned)
///         - Used to store messages 
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
    let account_info_iter = &mut accounts.iter();
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let layer_zero_messages_buffer_acc = next_account_info(account_info_iter)?;
    let execution_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let chain_admin_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Validate Provided accounts
    let (messages_bump, layer_zero_bump, execution_message_bump) =
        validate_accounts(
            program_id,
            messages_buffer_acc,
            layer_zero_messages_buffer_acc,
            execution_messages_buffer_acc,
            role_manager_acc,
            chain_admin_acc,
            system_program,
        )?;
    let rent = Rent::default();

    /**************************
     *  Message Buffer *
     *************************/
    if messages_buffer_acc.data_is_empty() {
        let messages_buffer_space = 10240;
        let required_lamports = rent.minimum_balance(messages_buffer_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            messages_buffer_acc.key,
            required_lamports,
            messages_buffer_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                messages_buffer_acc.clone(),
                system_program.clone(),
            ],
            &[&[MESSAGES_BUFFER_PREFIX.as_bytes(), &[messages_bump]]],
        )?;
    }

    // Update account data
    let messages_buffer_data = MessagesBuffer{
        is_initialized: true,
        message_nonce:0,
        chain_id: CHAIN_ID,
       messages: Vec::new(),
    };

    // Serialize account data
    messages_buffer_data
        .serialize(&mut &mut messages_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Message Buffer Initialized");


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
    messages_buffer_acc: &AccountInfo,
    layer_zero_messages_buffer_acc: &AccountInfo,
    execution_messages_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<(u8, u8, u8), ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate Account key and owner
    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;
    verify_owner(role_manager_acc, program_id)?;

    let (expected_messages_pda, messages_bump) = derive_messages_buffer(program_id);
    verify_derived_address(expected_messages_pda, messages_buffer_acc)?;

    let (expected_layer_zero_pda, layer_zero_bump) = derive_layer_zero_message_buffer(program_id);
    verify_derived_address(expected_layer_zero_pda, layer_zero_messages_buffer_acc)?;

    let (expected_execution_pda, execution_message_bump) =
        derive_execution_message_buffer(program_id);
    verify_derived_address(expected_execution_pda, execution_messages_buffer_acc)?;

    verify_system_program(system_program)?;

    // re-initialization guard for deposit buffer
    if !messages_buffer_acc.data_is_empty() {
        let deposit_buffer_data =
            MessagesBuffer::deserialize(&mut &messages_buffer_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;
        if deposit_buffer_data.is_initialized() {
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
        messages_bump,
        layer_zero_bump,
        execution_message_bump,
    ))
}