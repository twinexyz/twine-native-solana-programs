use crate::core::error::ProgramCustomError;
use crate::core::state::{NativeTokenVaultData, TokenDecimalMappings};
use crate::utils::constants::NATIVE_TOKEN_VAULT_DATA_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::clock::Clock;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    sysvar::Sysvar,
    pubkey::Pubkey,
    system_instruction,
};
use twine_chain::core::state::{DepositMessageInfo, DepositMessagesBuffer};
use twine_chain::utils::constants::DEPOSIT_BUFFER_PREFIX;

// Handles native token (SOL) deposits
pub fn native_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }

    if l1_token != "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    let account_info_iter = &mut accounts.iter();

    // Account[0]: the depositor (user) - must be a signer
    let user_account = next_account_info(account_info_iter)?;
    let native_token_vault_acc = next_account_info(account_info_iter)?;
    let native_token_vault_data_acc = next_account_info(account_info_iter)?;
    let deposit_messages_buffer_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if user_account.lamports() < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    if native_token_vault_data_acc.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let transfer_ix =
        system_instruction::transfer(user_account.key, native_token_vault_acc.key, amount);
    invoke(
        &transfer_ix,
        &[
            user_account.clone(),
            native_token_vault_acc.clone(),
            system_program.clone(),
        ],
    )?;
    let mut vault_data =
        NativeTokenVaultData::try_from_slice(&native_token_vault_data_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    vault_data.total_deposits = vault_data
        .total_deposits
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    vault_data
        .serialize(&mut *native_token_vault_data_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let token_decimal_mapping =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;

    let decimal_mapping = token_decimal_mapping
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    let (expected_deposit_pda, _) =
        Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], program_id);

    if expected_deposit_pda != *deposit_messages_buffer_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let deposit_message_buffer =
        DepositMessagesBuffer::try_from_slice(&deposit_messages_buffer_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = deposit_message_buffer.deposit_nonce + 1;

    let clock = Clock::get()?;

    let deposit_info = DepositMessageInfo {
        nonce: u64_nonce,
        chain_id: 900,
        slot_number: clock.slot,
        from_l1_pubkey: user_account.key.to_string(),
        to_twine_address: receiver_twine_address,
        l1_token,
        l2_token,
        amount: l2_amount,
    };

    let native_data_seeds = &[NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes()];
    let (_, native_data_bump) = Pubkey::find_program_address(native_data_seeds, program_id);

    //instructions number in TwineChainInstruction
    let discriminator: u8 = 5;

    let mut deposit_info_data = Vec::new();

    deposit_info
        .serialize(&mut deposit_info_data)
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let mut append_instruction_data = vec![discriminator];
    append_instruction_data.extend_from_slice(&deposit_info_data);

    let append_instruction_accounts = vec![
        AccountMeta::new(*deposit_messages_buffer_acc.key, false),
        AccountMeta::new_readonly(*role_manager_acc.key, false),
        AccountMeta::new_readonly(*native_token_vault_data_acc.key, true),
    ];

    let append_instruction = Instruction {
        program_id: *twine_chain_program.key,
        accounts: append_instruction_accounts,
        data: append_instruction_data,
    };

    invoke_signed(
        &append_instruction,
        &[
            deposit_messages_buffer_acc.clone(),
            role_manager_acc.clone(),
            native_token_vault_data_acc.clone(),
        ],
        &[&[
            NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes(),
            &[native_data_bump],
        ]],
    )?;
    msg!(
        "Native token deposit successful: {} lamports deposited",
        amount
    );

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
    use crate::core::state::{NativeTokenVaultData, TokenDecimalMapping, TokenDecimalMappings};
    use solana_program::{account_info::AccountInfo, clock::Epoch,pubkey::Pubkey, system_program};


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
    fn test_native_token_deposit_success() {
        let system_program_id = system_program::id();
        let program_id = Pubkey::new_unique();

        // Setup keys
        let user_key = Pubkey::new_unique();
        let native_token_vault_key = Pubkey::new_unique();
        let (native_token_vault_data_key, _) =
            Pubkey::find_program_address(&[NATIVE_TOKEN_VAULT_DATA_PREFIX.as_bytes()], &program_id);
        let (deposit_buffer_key, _) =
            Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], &program_id);
        let token_decimal_mappings_key = Pubkey::new_unique();
        let role_manager_key = Pubkey::new_unique();
        let twine_chain_program_id = Pubkey::new_unique();

        let mut native_token_vault_data = NativeTokenVaultData {
            is_initialized: true,
            total_deposits: 1000000,
        }
        .try_to_vec()
        .unwrap();

        let mut token_mappings_data = TokenDecimalMappings {
            is_initialized: true,
            mappings: vec![TokenDecimalMapping {
                l1_token: "11111111111111111111111111111111".to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                l1_decimals: 9,
                l2_decimals: 18,
            }],
        }
        .try_to_vec()
        .unwrap();

        let mut deposit_buffer_data = DepositMessagesBuffer {
            is_initialized: true,
            deposit_nonce: 5,
            deposit_messages: vec![],
        }
        .try_to_vec()
        .unwrap();

        let mut role_manager_data = vec![1; 1000];
        let mut user_data = vec![0; 100];
        let mut twine_chain_data = vec![0; 100];
        let mut system_program_data = vec![];

        // Setup lamports and owners
        let mut user_lamports = 1_000_000_000u64;
        let mut native_vault_lamports = 1_000_000u64;
        let mut native_vault_data_lamports = 1_000_000u64;
        let mut deposit_buffer_lamports = 1_000_000u64;
        let mut role_manager_lamports = 1_000_000u64;
        let mut token_mappings_lamports = 1_000_000u64;
        let mut twine_chain_lamports = 0u64;
        let mut system_program_lamports = 0;

        let mut user_owner = system_program::id();
        let mut native_vault_owner = system_program::id();
        let mut native_token_vault_data_owner = program_id;
        let mut deposit_buffer_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut token_mappings_owner = program_id;
        let mut twine_chain_owner = system_program::id();
        let mut system_program_owner = system_program::id();

        // Create accounts
        let user_account = create_test_account(
            &user_key,
            true,
            false,
            &mut user_lamports,
            &mut user_data,
            &mut user_owner,
        );

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
            &mut native_token_vault_data,
            &mut native_token_vault_data_owner,
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

        let deposit_buffer_account = create_test_account(
            &deposit_buffer_key,
            false,
            true,
            &mut deposit_buffer_lamports,
            &mut deposit_buffer_data,
            &mut deposit_buffer_owner,
        );

        let twine_chain_program_account = create_test_account(
            &twine_chain_program_id,
            false,
            false,
            &mut twine_chain_lamports,
            &mut twine_chain_data,
            &mut twine_chain_owner,
        );

        let system_program_account = create_test_account(
            &system_program_id,
            false,
            false,
            &mut system_program_lamports,
            &mut system_program_data,
            &mut system_program_owner,
        );

        let accounts = vec![
            user_account,
            native_token_vault_account,
            native_token_vault_data_account,
            deposit_buffer_account,
            token_decimal_mappings_account,
            role_manager_account,
            system_program_account,
            twine_chain_program_account,
        ];

        let result = native_token_deposit(
            &program_id,
            &accounts,
            "0x1234567890123456789012345678901234567890".to_string(),
            "11111111111111111111111111111111".to_string(),
            "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
            500_000,
        );

        assert!(result.is_ok());
    }
}
