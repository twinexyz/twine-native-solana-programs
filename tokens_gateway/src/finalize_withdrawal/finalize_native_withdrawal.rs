use crate::core::error::ProgramCustomError;
use crate::core::state::{
    ExecutedWithdrawals, FinalizeInputWithdrawal, NativeTokenVaultData, TokenDecimalMappings,
};
use crate::utils::constants::NATIVE_TOKEN_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::sysvar::clock::Clock;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use sp1_solana::verify_proof;

pub fn finalize_native_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdrawal_inputs: FinalizeInputWithdrawal,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let user = next_account_info(account_iter)?;
    let native_token_vault_acc = next_account_info(account_iter)?;
    let native_token_vault_data_acc = next_account_info(account_iter)?;
    let execution_message_buffer_acc = next_account_info(account_iter)?;
    let twine_chain_storage_acc = next_account_info(account_iter)?;
    let forced_withdrawal_messages_buffer_acc = next_account_info(account_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_iter)?;
    let receiver_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;
    let twine_chain_program = next_account_info(account_iter)?;
    let sp_verifier_program = next_account_info(account_iter)?;

    if withdrawal_inputs.public_input.batch_number > 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    let amount = TokenDecimalMappings::parse_amount_to_u64(&withdrawal_inputs.public_input.amount)?;
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if native_token_vault_acc.lamports() >= amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    }

    if withdrawal_inputs.public_input.l1_token_address != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&withdrawal_inputs.public_input.l2_token_address)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if withdrawal_inputs.public_input.l1_receiver_address != receiver_acc.key.to_string() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let token_decimal_mappings =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;
    let decimal_mapping = token_decimal_mappings
        .get_mapping(&withdrawal_inputs.public_input.l1_token_address)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;
    let converted_amount = TokenDecimalMappings::convert_l2_to_l1(
        &withdrawal_inputs.public_input.amount,
        decimal_mapping.l2_decimals,
        decimal_mapping.l1_decimals,
    )?;
    let actual_amount = TokenDecimalMappings::parse_amount_to_u64(&converted_amount)?;
    let mut flag = false;

    if withdrawal_inputs.public_input.is_forced_withdrawal == 1 {
        // Check if the withdrawal is present in execution message buffer
        // for withdrawals in forced_withdrawal_messages_buffer_acc.withdraw_messages.clone() {
        //     if withdrawal_inputs.public_input.nonce == withdrawals.nonce {
        //         flag = true;
        //         break;
        //     }
        // }
        if flag == true {
            // Native token (SOL) withdrawal
            process_native_token_withdrawal(
                &program_id,
                &native_token_vault_acc,
                &native_token_vault_data_acc,
                &system_program,
                &receiver_acc,
                actual_amount,
            )?;
        } else {
            let mut executed_withdrawal_buffer = ExecutedWithdrawals::try_from_slice(
                &executed_withdrawals_buffer_acc.data.borrow(),
            )?;
            // For L2 initiated withdrawals
            if withdrawal_inputs.public_input.nonce
                < executed_withdrawal_buffer.withdrawal_nonce_lower_bound
            {
                return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
            };

            if !executed_withdrawal_buffer
                .executed_withdrawal_nonces
                .contains(&withdrawal_inputs.public_input.nonce)
            {
                return Err(ProgramCustomError::WithdrawalAlreadyExecuted.into());
            };
            // Native token (SOL) withdrawal
            process_native_token_withdrawal(
                &program_id,
                &native_token_vault_acc,
                &native_token_vault_data_acc,
                &system_program,
                &receiver_acc,
                actual_amount,
            )?;
            executed_withdrawal_buffer
                .executed_withdrawal_nonces
                .push(withdrawal_inputs.public_input.nonce);
            executed_withdrawal_buffer.post_withdrawal_processing();
        }
    }
    Ok(())
}

fn process_native_token_withdrawal<'info>(
    program_id: &Pubkey,
    native_token_vault: &AccountInfo<'info>,
    native_token_vault_data: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    receiver: &AccountInfo<'info>,
    amount: u64,
) -> ProgramResult {
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if native_token_vault.lamports() >= amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    };
    let native_vault_seeds = &[NATIVE_TOKEN_PREFIX.as_bytes()];
    let (native_vault_key, native_vault_bump) =
        Pubkey::find_program_address(native_vault_seeds, program_id);

    if native_vault_key != *native_token_vault.key {
        return Err(ProgramCustomError::InvalidAccount.into());
    }
    let seeds = &[NATIVE_TOKEN_PREFIX.as_bytes(), &[native_vault_bump]];
    let signer_seeds = &[&seeds[..]];
    // Create transfer instruction
    let transfer_instruction =
        solana_program::system_instruction::transfer(native_token_vault.key, &receiver.key, amount);
    // Perform the transfer using invoke_signed
    invoke_signed(
        &transfer_instruction,
        &[
            native_token_vault.clone(),
            receiver.clone(),
            system_program.clone(),
        ],
        signer_seeds,
    )?;
    let mut vault_data =
        NativeTokenVaultData::try_from_slice(&native_token_vault_data.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    vault_data
        .serialize(&mut *native_token_vault_data.data.borrow_mut())
        .map_err(|_| ProgramError::AccountDataTooSmall)?;

    let clock = Clock::get()?;

    Ok(())
}
