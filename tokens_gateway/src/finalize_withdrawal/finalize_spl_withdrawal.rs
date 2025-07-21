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
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use spl_token::instruction as token_instruction;
use twine_chain::core::state::{ExecutionMessageBuffer, TwineChainStorage};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{
            ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal, SplTokensVaultData,
            TokenDecimalMappings,
        },
    },
    utils::{
        constants::{SPL_AUTH_PREFIX, SPL_TOKENS_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn finalize_spl_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdrawal_inputs: FinalizeInputWithdrawal,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let _user = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let vault_authority_acc = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let execution_message_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_storage_acc = next_account_info(account_info_iter)?;
    let executed_withdrawals_buffer_acc = next_account_info(account_info_iter)?;
    let receiver_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    // Validate inputs
    if withdrawal_inputs.public_input.block_number == 0 {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if withdrawal_inputs.public_input.l1_token_address != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidArgument.into());
    }
    if withdrawal_inputs.public_input.l1_token_address == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&withdrawal_inputs.public_input.l2_token_address)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if withdrawal_inputs.public_input.l1_receiver_address != receiver_acc.key.to_string() {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let twine_chain_storage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // if withdrawal_inputs.public_input.block_number
    //     > twine_chain_storage.last_finalized_batch.end_block
    // {
    //     return Err(ProgramCustomError::BatchNotFinalized.into());
    // };

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
        let execution_message_buffer = ExecutionMessageBuffer::deserialize(
            &mut &execution_message_buffer_acc.data.borrow()[..],
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;
        // Check if the withdrawal is present in execution message buffer
        // for withdrawals in execution_message_buffer.withdrawals.clone() {
        //     if withdrawal_inputs.public_input.nonce == withdrawals.nonce {
        //         flag = true;
        //         break;
        //     }
        // }

        flag = true;
        if flag == true {
            //Spl Token withdrawal
            process_spl_token_withdrawal(
                &program_id,
                &spl_tokens_vault_acc,
                &spl_tokens_vault_data_acc,
                &vault_authority_acc,
                &mint,
                &token_program,
                &receiver_acc,
                actual_amount,
            )?;

            let spl_data_seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()];
            let (_, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);

            //instructions number in TwineChainInstruction
            let discriminator: u8 = 7;
            let remove_message_instruction_data = vec![discriminator];
            let remove_message_instruction_accounts = vec![
                AccountMeta::new(*execution_message_buffer_acc.key, false),
                AccountMeta::new_readonly(*role_manager_acc.key, false),
                AccountMeta::new_readonly(*spl_tokens_vault_data_acc.key, true),
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
                    role_manager_acc.clone(),
                    spl_tokens_vault_data_acc.clone(),
                ],
                &[&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
            )?;
        }
    } else {
        let executed_withdrawal_buffer = ExecutedWithdrawalsBuffer::try_from_slice(
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
        // Spl Token withdrawal
        process_spl_token_withdrawal(
            &program_id,
            &spl_tokens_vault_acc,
            &spl_tokens_vault_data_acc,
            &vault_authority_acc,
            &mint,
            &token_program,
            &receiver_acc,
            actual_amount,
        )?;
    }

    let clock = Clock::get()?;

    msg!(
    "EVENT:SPL_WITHDRAWAL_SUCCESSFUL: nonce={}, l1_receiver_address={}, l1_token_address={}, l2_token_address={}, chain_id={}, amount={}, slot={}",
    withdrawal_inputs.public_input.nonce,
    withdrawal_inputs.public_input.l1_receiver_address,
    withdrawal_inputs.public_input.l1_token_address,
    withdrawal_inputs.public_input.l2_token_address,
    withdrawal_inputs.public_input.chain_id,
    actual_amount,
    clock.slot
);

    Ok(())
}

fn process_spl_token_withdrawal<'info>(
    program_id: &Pubkey,
    spl_tokens_vault: &AccountInfo<'info>,
    spl_tokens_vault_data_acc: &AccountInfo<'info>,
    vault_authority: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    receiver: &AccountInfo<'info>,
    amount: u64,
) -> ProgramResult {
    if amount <= 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }

    let vault_token_account =
        spl_token::state::Account::unpack(&mut &spl_tokens_vault.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if vault_token_account.amount < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    let spl_data_seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()];
    let (spl_data_key, _) = Pubkey::find_program_address(spl_data_seeds, program_id);

    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let spl_vault_seeds = &[SPL_AUTH_PREFIX.as_bytes()];
    let (_, spl_vault_bump) = Pubkey::find_program_address(spl_vault_seeds, program_id);
    let seeds = &[SPL_AUTH_PREFIX.as_bytes(), &[spl_vault_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_instruction = token_instruction::transfer(
        &spl_token::id(),
        &spl_tokens_vault.key,
        &receiver.key,
        &vault_authority.key,
        &[],
        amount,
    )?;

    invoke_signed(
        &transfer_instruction,
        &[
            spl_tokens_vault.clone(),
            receiver.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        signer_seeds,
    )?;

    let mut spl_tokens_vault_data =
        SplTokensVaultData::deserialize(&mut &spl_tokens_vault_data_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    spl_tokens_vault_data.update_withdraw(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut &mut spl_tokens_vault_data_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

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
        ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal, ReceiptCommitment, SplTokensVaultData,
        TokenDecimalMappingData, TokenDecimalMappings, TokenDepositData,
    };
    use crate::utils::constants::{
        EXECUTED_WITHDRAWALS_BUFFER_PREFIX, SPL_AUTH_PREFIX, SPL_TOKENS_VAULT_DATA_PREFIX,
    };
    use solana_program::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, system_program};
    use spl_token::state::{Account as TokenAccount, Mint};
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
    fn create_spl_token_account_data(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0u8; TokenAccount::LEN];
        let token_account = TokenAccount {
            mint: *mint,
            owner: *owner,
            amount,
            delegate: Default::default(),
            state: spl_token::state::AccountState::Initialized,
            is_native: Default::default(),
            delegated_amount: 0,
            close_authority: Default::default(),
        };
        TokenAccount::pack(token_account, &mut data).unwrap();
        data
    }
    fn create_mint_data(decimals: u8) -> Vec<u8> {
        let mut data = vec![0u8; Mint::LEN];
        let mint = Mint {
            mint_authority: Default::default(),
            supply: 1_000_000_000,
            decimals,
            is_initialized: true,
            freeze_authority: Default::default(),
        };
        Mint::pack(mint, &mut data).unwrap();
        data
    }

    #[test]
    fn test_spl_native_withdrawal_success() {
        let program_id = Pubkey::new_unique();
        let token_program_id = spl_token::id();
        let twine_chain_program_id = Pubkey::new_unique();

        // Setup keys
        let user_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let receiver_key = Pubkey::new_unique();
        let role_manager_key = Pubkey::new_unique();
        let spl_tokens_vault_key = Pubkey::new_unique();

        let (spl_tokens_vault_data_key, _) =
            Pubkey::find_program_address(&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()], &program_id);

        let (vault_authority_key, _) =
            Pubkey::find_program_address(&[SPL_AUTH_PREFIX.as_bytes()], &program_id);

        let (execution_message_buffer_key, _) = Pubkey::find_program_address(
            &[EXECUTION_MESSAGE_BUFFER_PREFIX.as_bytes()],
            &twine_chain_program_id,
        );
        let (twine_chain_storage_key, _) = Pubkey::find_program_address(
            &[TWINE_CHAIN_STORAGE_PREFIX.as_bytes()],
            &twine_chain_program_id,
        );
        let (executed_withdrawals_buffer_key, _) = Pubkey::find_program_address(
            &[EXECUTED_WITHDRAWALS_BUFFER_PREFIX.as_bytes()],
            &program_id,
        );
        let token_decimal_mappings_key = Pubkey::new_unique();

        let mut spl_vault_data = SplTokensVaultData {
            is_initialized: true,
            total_deposited_amount: vec![TokenDepositData {
                token_id: Pubkey::new_unique(),
                amount: 0,
            }],
        }
        .try_to_vec()
        .unwrap();

        let mut spl_vault_token_data =
            create_spl_token_account_data(&mint_key, &vault_authority_key, 2_000_000);
        let mut mint_data = create_mint_data(6);
        let mut receiver_token_data = create_spl_token_account_data(&mint_key, &receiver_key, 0);

        let mut execution_buffer = ExecutionMessageBuffer {
            is_initialized: true,
            withdrawals: vec![ForcedWithdrawMessageInfo {
                nonce: 456,
                chain_id: 900,
                slot_number: 1000,
                from_twine_address: "0x1234567890123456789012345678901234567890".to_string(),
                to_l1_pubkey: receiver_key.to_string(),
                l1_token: mint_key.to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                amount: "1000000000000000000".to_string(), // 18 decimals
            }],
        }
        .try_to_vec()
        .unwrap();
        let mut twine_storage = TwineChainStorage {
            is_initialized: true,
            last_copied_deposit_nonce:0,
            last_copied_forced_withdrawal_nonce:0,
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
                l1_token: mint_key.to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                l1_decimals: 6,
                l2_decimals: 18,
            }],
        }
        .try_to_vec()
        .unwrap();

        // Setup lamports and owners
        let mut user_lamports = 1_000_000u64;
        let mut spl_vault_data_lamports = 1_000_000u64;
        let mut spl_vault_lamports = 1_000_000u64;
        let mut vault_authority_lamports = 1_000_000u64;
        let mut token_program_lamports = 0u64;
        let mut mint_lamports = 1_000_000u64;
        let mut execution_buffer_lamports = 1_000_000u64;
        let mut twine_storage_lamports = 1_000_000u64;
        let mut executed_withdrawals_lamports = 1_000_000u64;
        let mut receiver_lamports = 1_000_000u64;
        let mut role_manager_lamports = 1_000_000u64;
        let mut token_mappings_lamports = 1_000_000u64;
        let mut twine_chain_lamports = 0u64;

        let mut user_owner = system_program::id();
        let mut spl_vault_data_owner = program_id;
        let mut spl_vault_owner = token_program_id;
        let mut vault_authority_owner = system_program::id();
        let mut token_program_owner = system_program::id();
        let mut mint_owner = token_program_id;
        let mut execution_buffer_owner = twine_chain_program_id;
        let mut twine_storage_owner = twine_chain_program_id;
        let mut executed_withdrawals_owner = program_id;
        let mut receiver_owner = token_program_id;
        let mut role_manager_owner = program_id;
        let mut token_mappings_owner = program_id;
        let mut twine_chain_owner = system_program::id();

        // Create dummy data for simple accounts
        let mut user_data = vec![0; 100];
        let mut vault_authority_data = vec![0; 100];
        let mut token_program_data = vec![];
        let mut role_manager_data = vec![1; 1000];
        let mut twine_chain_data = vec![0; 100];

        // Create account info structs
        let user_account = create_test_account(
            &user_key,
            true,
            false,
            &mut user_lamports,
            &mut user_data,
            &mut user_owner,
        );

        let spl_tokens_vault_data_account = create_test_account(
            &spl_tokens_vault_data_key,
            false,
            true,
            &mut spl_vault_data_lamports,
            &mut spl_vault_data,
            &mut spl_vault_data_owner,
        );

        let spl_tokens_vault_account = create_test_account(
            &spl_tokens_vault_key,
            false,
            true,
            &mut spl_vault_lamports,
            &mut spl_vault_token_data,
            &mut spl_vault_owner,
        );

        let vault_authority_account = create_test_account(
            &vault_authority_key,
            false,
            false,
            &mut vault_authority_lamports,
            &mut vault_authority_data,
            &mut vault_authority_owner,
        );

        let token_program_account = create_test_account(
            &token_program_id,
            false,
            false,
            &mut token_program_lamports,
            &mut token_program_data,
            &mut token_program_owner,
        );

        let mint_account = create_test_account(
            &mint_key,
            false,
            false,
            &mut mint_lamports,
            &mut mint_data,
            &mut mint_owner,
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
            &mut receiver_token_data,
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

        let twine_chain_program_account = create_test_account(
            &twine_chain_program_id,
            false,
            false,
            &mut twine_chain_lamports,
            &mut twine_chain_data,
            &mut twine_chain_owner,
        );

        let accounts = vec![
            user_account,
            spl_tokens_vault_data_account,
            spl_tokens_vault_account,
            vault_authority_account,
            token_program_account,
            mint_account,
            execution_message_buffer_account,
            twine_chain_storage_account,
            executed_withdrawals_buffer_account,
            receiver_account,
            role_manager_account,
            token_decimal_mappings_account,
            twine_chain_program_account,
        ];

        let withdrawal_inputs = FinalizeInputWithdrawal {
            public_input: ReceiptCommitment {
                chain_id: 900,
                block_number: 10,
                nonce: 456,
                is_forced_withdrawal: 1,
                receipt_root: [0u8; 32],
                l1_receiver_address: receiver_key.to_string(),
                l1_token_address: mint_key.to_string(),
                l2_token_address: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                amount: "1000000000000000000".to_string(),
            },
            inclusion_proof: vec![0u8; 256],
        };

        let result = finalize_spl_withdrawal(&program_id, &accounts, withdrawal_inputs);
        assert!(
            result.is_ok(),
            "Forced SPL withdrawal should succeed: {:?}",
            result.err()
        );
    }
}
