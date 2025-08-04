use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::sysvar::clock::Clock;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use twine_chain::core::{instruction::TwineChainInstruction, state::{ExecutionMessageBuffer, TwineChainStorage}};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal, NativeTokenVaultData,
            TokenDecimalMappings,
        },
    },
    utils::{
        address_derivation::derive_native_token_vault_data,
        constants::{NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn refund_native_token(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdrawal_inputs: FinalizeInputWithdrawal,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let execution_message_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    if withdrawal_inputs.public_input.block_number <= 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    let amount = TokenDecimalMappings::parse_amount_to_u64(&withdrawal_inputs.public_input.amount)?;
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if native_token_vault_acc.lamports() <= amount {
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

    // Deserialize twine_chain_storage_acc
    let twine_chain_storage = {
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    // if withdrawal_inputs.public_input.block_number
    //     > twine_chain_storage.last_finalized_batch.end_block
    // {
    //     return Err(ProgramCustomError::BatchNotFinalized.into());
    // };

    // encoding public input structure to get public input
    if !twine_chain_storage.skip_verification {
        let public_input = withdrawal_inputs.public_input.abi_encode_packed();
        verify_proof(
            &withdrawal_inputs.inclusion_proof,
            &public_input,
            &twine_chain_storage.execution_vkey,
            GROTH16_VK_4_0_0_RC3_BYTES,
        )
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    }

    let token_decimal_mappings =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

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
        let execution_message_buffer =
            ExecutionMessageBuffer::deserialize(&mut &execution_message_buffer_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;

        for withdrawals in execution_message_buffer.withdrawals.clone() {
            if withdrawal_inputs.public_input.nonce == withdrawals.nonce {
                flag = true;
                break;
            }
        }

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

            let payload = TwineChainInstruction::RemoveWithdrawalMessage {
                nonce: withdrawal_inputs.public_input.nonce,
            };

            let mut remove_message_instruction_data = vec![];
            remove_message_instruction_data.extend(payload.try_to_vec().unwrap());

            let remove_message_instruction_accounts = vec![
                AccountMeta::new(*execution_message_buffer_acc.key, false),
                AccountMeta::new_readonly(*role_manager.key, false),
                AccountMeta::new_readonly(*native_token_vault_data_acc.key, true),
            ];

            let remove_message_instruction = Instruction {
                program_id: *twine_chain_program.key,
                accounts: remove_message_instruction_accounts,
                data: remove_message_instruction_data,
            };

            let (_, native_data_bump) = derive_native_token_vault_data(&program_id);
            let seeds = &[
                NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
                &[native_data_bump],
            ];
            let signer_seeds = &[&seeds[..]];

            invoke_signed(
                &remove_message_instruction,
                &[
                    execution_message_buffer_acc.clone(),
                    role_manager.clone(),
                    native_token_vault_data_acc.clone(),
                ],
                signer_seeds,
            )?;
        }
    } else {
        let mut executed_withdrawal_buffer = ExecutedWithdrawalsBuffer::try_from_slice(
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
    let clock = Clock::get()?;

    msg!(
    "EVENT:NATIVE_WITHDRAWAL_SUCCESSFUL: nonce={}, l1_receiver_address={}, l1_token_address={}, chain_id={}, amount={}, slot={}",
    withdrawal_inputs.public_input.nonce,
    withdrawal_inputs.public_input.l1_receiver_address,
    withdrawal_inputs.public_input.l1_token_address,
    withdrawal_inputs.public_input.chain_id,
    actual_amount,
    clock.slot
);
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
    println!("Before lamports check");

    if native_token_vault.lamports() <= amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    };
        println!("After lamports check");

    let native_vault_seeds = &[NATIVE_TOKEN_VAULT_PREFIX.as_bytes()];
    let (native_vault_key, native_vault_bump) =
        Pubkey::find_program_address(native_vault_seeds, program_id);

    if native_vault_key != *native_token_vault.key {
        return Err(ProgramCustomError::InvalidAccount.into());
    }

    let seeds = &[NATIVE_TOKEN_VAULT_PREFIX.as_bytes(), &[native_vault_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_instruction =
        solana_program::system_instruction::transfer(native_token_vault.key, &receiver.key, amount);

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
        NativeTokenVaultData::deserialize(&mut &native_token_vault_data.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    vault_data
        .serialize(&mut &mut native_token_vault_data.data.borrow_mut()[..])
        .map_err(|_| ProgramError::AccountDataTooSmall)?;

    Ok(())
}