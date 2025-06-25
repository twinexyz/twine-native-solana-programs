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
use twine_chain::core::state::TwineChainStorage;

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal, NativeTokenVaultData,
            TokenDecimalMappings,
        },
    },
    utils::{
        constants::{NATIVE_TOKEN_VAULT_DATA_PREFIX, NATIVE_TOKEN_VAULT_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn finalize_native_withdrawal(
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
            TokenDecimalMappings::deserialize(&mut &execution_message_buffer_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;

        // for withdrawals in execution_message_buffer.withdrawals.clone() {
        //     if withdrawal_inputs.public_input.nonce == withdrawals.nonce {
        //         flag = true;
        //         break;
        //     }
        // }

        flag = true;
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

            let native_data_seeds = &[NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes()];
            let (_, native_data_bump) = Pubkey::find_program_address(native_data_seeds, program_id);

            //instructions number in TwineChainInstruction
            let discriminator: u8 = 7;
            let remove_message_instruction_data = vec![discriminator];
            let remove_message_instruction_accounts = vec![
                AccountMeta::new(*execution_message_buffer_acc.key, false),
                AccountMeta::new_readonly(*role_manager.key, false),
                AccountMeta::new_readonly(*native_token_vault_acc.key, true),
            ];

            let remove_message_instruction = Instruction {
                program_id: *twine_chain_program.key,
                accounts: remove_message_instruction_accounts,
                data: remove_message_instruction_data,
            };

            invoke_signed(
                &remove_message_instruction,
                &[
                    execution_message_buffer_acc.clone(),
                    role_manager.clone(),
                    native_token_vault_acc.clone(),
                ],
                &[&[
                    NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
                    &[native_data_bump],
                ]],
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
    if native_token_vault.lamports() <= amount {
        return Err(ProgramCustomError::InsufficientFunds.into());
    };
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

#[cfg(test)]
use mock_clock::Clock;

#[cfg(test)]
mod mock_clock {
    use solana_program::program_error::ProgramError;

    pub struct Clock {
        pub slot: u64,
    }

    impl Clock {
        pub fn get() -> Result<Clock, ProgramError> {
            Ok(Clock { slot: 1000 })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{
        ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal, NativeTokenVaultData,
        ReceiptCommitment, TokenDecimalMappingData, TokenDecimalMappings,
    };
    use crate::utils::constants::EXECUTED_WITHDRAWALS_BUFFER_PREFIX;
    use solana_program::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, system_program};
    use twine_chain::core::state::{
        BatchInfo, ExecutionMessageBuffer, ForcedWithdrawMessageInfo, TwineChainStorage,
    };
    use twine_chain::utils::constants::{
        EXECUTION_MESSAGE_BUFFER_PREFIX, TWINE_CHAIN_STORAGE_PREFIX,
    };

    fn create_test_account<'a>(
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
    fn test_finalize_native_withdrawal_success() {
        let program_id = Pubkey::new_unique();
        let system_program_id = system_program::id();
        let twine_chain_program_id = Pubkey::new_unique();

        // Setup keys
        let (native_token_vault_key, _) =
            Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_PREFIX.as_bytes()], &program_id);
        let (native_token_vault_data_key, _) =
            Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes()], &program_id);
        let (execution_message_buffer_key, _) = Pubkey::find_program_address(
            &[EXECUTION_MESSAGE_BUFFER_PREFIX.as_bytes()],
            &program_id,
        );
        let (twine_chain_storage_key, _) =
            Pubkey::find_program_address(&[TWINE_CHAIN_STORAGE_PREFIX.as_bytes()], &program_id);
        let (executed_withdrawals_buffer_key, _) = Pubkey::find_program_address(
            &[EXECUTED_WITHDRAWALS_BUFFER_PREFIX.as_bytes()],
            &program_id,
        );
        let receiver_key = Pubkey::new_unique();
        let role_manager_key = Pubkey::new_unique();
        let token_decimal_mappings_key = Pubkey::new_unique();

        let mut native_vault_data = NativeTokenVaultData {
            is_initialized: true,
            total_deposits: 2_000_000,
        }
        .try_to_vec()
        .unwrap();

        let mut execution_buffer = ExecutionMessageBuffer {
            is_initialized: true,
            withdrawals: vec![ForcedWithdrawMessageInfo {
                nonce: 123,
                chain_id: 900,
                slot_number: 1000,
                from_twine_address: "0x1234567890123456789012345678901234567890".to_string(),
                to_l1_pubkey: receiver_key.to_string(),
                l1_token: "11111111111111111111111111111111".to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                amount: "1000000000000000000".to_string(), // 18 decimals
            }],
        }
        .try_to_vec()
        .unwrap();

        let mut twine_storage = TwineChainStorage {
            is_initialized: true,
            groth16_vk: vec![0u8; 32],
            execution_vkey: "execution_vkey_string".to_string(),
            inclusion_vkey: "inclusion_vkey_string".to_string(),
            withdrawal_vkey: "withdrawal_vkey_string".to_string(),
            skip_verification: true,
            last_finalized_batch: BatchInfo {
                start_block: 0,
                end_block: 100,
            },
            last_committed_batch: BatchInfo {
                start_block: 0,
                end_block: 0,
            },
            last_transaction_finalized_batch: BatchInfo {
                start_block: 0,
                end_block: 0,
            },
            last_finalized_receipt_root: [1u8; 32],
        }
        .try_to_vec()
        .unwrap();

        let mut executed_withdrawals = ExecutedWithdrawalsBuffer {
            is_initialized: true,
            withdrawal_nonce_lower_bound: 0,
            executed_withdrawal_nonces: vec![],
        }
        .try_to_vec()
        .unwrap();

        let mut token_mappings_data = TokenDecimalMappings {
            is_initialized: true,
            mappings: vec![TokenDecimalMappingData {
                l1_token: "11111111111111111111111111111111".to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                l1_decimals: 9,
                l2_decimals: 18,
            }],
        }
        .try_to_vec()
        .unwrap();

        // Setup lamports and owners
        let mut native_vault_lamports = 2_000_000u64;
        let mut native_vault_data_lamports = 1_000_000u64;
        let mut execution_buffer_lamports = 1_000_000u64;
        let mut twine_storage_lamports = 1_000_000u64;
        let mut executed_withdrawals_lamports = 1_000_000u64;
        let mut receiver_lamports = 1_000_000u64;
        let mut role_manager_lamports = 1_000_000u64;
        let mut token_mappings_lamports = 1_000_000u64;
        let mut system_program_lamports = 0u64;
        let mut twine_chain_lamports = 0u64;

        let mut native_vault_owner = system_program_id;
        let mut native_vault_data_owner = program_id;
        let mut execution_buffer_owner = program_id;
        let mut twine_storage_owner = program_id;
        let mut executed_withdrawals_owner = program_id;
        let mut receiver_owner = system_program_id;
        let mut role_manager_owner = program_id;
        let mut token_mappings_owner = program_id;
        let mut system_program_owner = system_program_id;
        let mut twine_chain_owner = system_program_id;

        // Create dummy data for accounts that don't need specific data
        let mut role_manager_data = vec![1; 1000];
        let mut receiver_data = vec![0; 100];
        let mut system_program_data = vec![];
        let mut twine_chain_data = vec![0; 100];

        let native_token_vault_account = create_test_account(
            &native_token_vault_key,
            false,
            true,
            &mut native_vault_lamports,
            &mut [],
            &mut native_vault_owner,
        );

        let native_token_vault_data_account = create_test_account(
            &native_token_vault_data_key,
            false,
            true,
            &mut native_vault_data_lamports,
            &mut native_vault_data,
            &mut native_vault_data_owner,
        );

        let execution_message_buffer_account = create_test_account(
            &execution_message_buffer_key,
            false,
            true,
            &mut execution_buffer_lamports,
            &mut execution_buffer,
            &mut execution_buffer_owner,
        );

        let twine_chain_storage_account = create_test_account(
            &twine_chain_storage_key,
            false,
            false,
            &mut twine_storage_lamports,
            &mut twine_storage,
            &mut twine_storage_owner,
        );

        let executed_withdrawals_buffer_account = create_test_account(
            &executed_withdrawals_buffer_key,
            false,
            true,
            &mut executed_withdrawals_lamports,
            &mut executed_withdrawals,
            &mut executed_withdrawals_owner,
        );

        let receiver_account = create_test_account(
            &receiver_key,
            false,
            true,
            &mut receiver_lamports,
            &mut receiver_data,
            &mut receiver_owner,
        );

        let role_manager_account = create_test_account(
            &role_manager_key,
            false,
            false,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        let token_decimal_mappings_account = create_test_account(
            &token_decimal_mappings_key,
            false,
            false,
            &mut token_mappings_lamports,
            &mut token_mappings_data,
            &mut token_mappings_owner,
        );

        let system_program_account = create_test_account(
            &system_program_id,
            false,
            false,
            &mut system_program_lamports,
            &mut system_program_data,
            &mut system_program_owner,
        );

        let twine_chain_program_account = create_test_account(
            &twine_chain_program_id,
            false,
            false,
            &mut twine_chain_lamports,
            &mut twine_chain_data,
            &mut twine_chain_owner,
        );

        let accounts = vec![
            native_token_vault_account,
            native_token_vault_data_account,
            execution_message_buffer_account,
            twine_chain_storage_account,
            executed_withdrawals_buffer_account,
            receiver_account,
            role_manager_account,
            token_decimal_mappings_account,
            system_program_account,
            twine_chain_program_account,
        ];

        let withdrawal_inputs = FinalizeInputWithdrawal {
            public_input: ReceiptCommitment {
                chain_id: 900,
                block_number: 10,
                nonce: 123,
                is_forced_withdrawal: 1,
                receipt_root: [0u8; 32],
                l1_receiver_address: receiver_key.to_string(),
                l1_token_address: "11111111111111111111111111111111".to_string(),
                l2_token_address: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                amount: "1000000000000000000".to_string(),
            },
            inclusion_proof: vec![0u8; 256],
        };

        let result = finalize_native_withdrawal(&program_id, &accounts, withdrawal_inputs);
        assert!(
            result.is_ok(),
            "Forced withdrawal should succeed: {:?}",
            result.err()
        );
    }
}
