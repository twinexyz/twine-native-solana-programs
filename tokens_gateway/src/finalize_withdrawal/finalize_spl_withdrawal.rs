use crate::core::error::ProgramCustomError;
use crate::core::state::{
    ExecutedWithdrawalsBuffer, FinalizeInputWithdrawal, SplTokensVaultData, TokenDecimalMappings,
};
use crate::utils::constants::SPL_DATA_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::sysvar::clock::Clock;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
};
use sp1_solana::{verify_proof, GROTH16_VK_4_0_0_RC3_BYTES};
use spl_token::instruction as token_instruction;
use twine_chain::core::state::{ExecutionMessageBuffer, TwineChainStorage};

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
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;

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
    // Deserialize twine_chain_storage_acc
    let twine_chain_storage =
        TwineChainStorage::try_from_slice(&twine_chain_storage_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if withdrawal_inputs.public_input.block_number
        <= twine_chain_storage.last_finalized_batch.end_block
    {
        return Err(ProgramCustomError::BatchNotFinalized.into());
    };
    // encoding public input structure to get public input
    let public_input = withdrawal_inputs.public_input.abi_encode_packed();
    verify_proof(
        &withdrawal_inputs.inclusion_proof,
        &public_input,
        &twine_chain_storage.execution_vkey,
        GROTH16_VK_4_0_0_RC3_BYTES,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

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
        let execution_message_buffer =
            ExecutionMessageBuffer::try_from_slice(&execution_message_buffer_acc.data.borrow())
                .map_err(|_| ProgramError::InvalidAccountData)?;
        // Check if the withdrawal is present in execution message buffer
        for withdrawals in execution_message_buffer.withdrawals.clone() {
            if withdrawal_inputs.public_input.nonce == withdrawals.nonce {
                flag = true;
                break;
            }
        }
        if !flag {
            return Err(ProgramCustomError::NonceNotFound.into());
        }
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
    }
    let clock = Clock::get()?;

    msg!(
        "EVENT:SPL_WITHDRAWAL_SUCCESSFUL:{}:{}:{}:{}:{}:{}:{}",
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
    let vault_token_account = spl_token::state::Account::unpack(&spl_tokens_vault.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    if vault_token_account.amount < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    let spl_data_seeds = &[SPL_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let seeds = &[SPL_DATA_PREFIX.as_bytes(), &[spl_data_bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_instruction = token_instruction::transfer(
        token_program.key,
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
            token_program.clone(),
            vault_authority.clone(),
        ],
        signer_seeds,
    )?;
    let mut spl_tokens_vault_data =
        SplTokensVaultData::try_from_slice(&spl_tokens_vault_data_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    spl_tokens_vault_data.update_withdraw(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut *spl_tokens_vault_data_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;
    Ok(())
}
