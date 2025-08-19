use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::clock::Clock;
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
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::str::FromStr;
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{ForcedWithdrawMessageInfo,MessagesBuffer,TransactionType},
    },
    ID as twine_chain_program_id,
};

#[cfg(not(test))]
use crate::utils::recover_address::recover_address;

use crate::{
    core::{
        error::ProgramCustomError,
        state::{SignMessageInfo, TokenDecimalMappings},
    },
    utils::{
        constants::{CHAIN_ID, SPL_TOKENS_VAULT_DATA_PREFIX,FORCED_WITHDRAW_TRANSACTION},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn forced_spl_token_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if !is_valid_ethereum_address(&from_twine_address)? {
        return Err(ProgramCustomError::InvalidArgument.into());
    }

    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let to_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let forced_withdrawal_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    if l1_token != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidArgument.into());
    }

    let parsed_to_pubkey =
        Pubkey::from_str(&to_l1_pubkey).map_err(|_| ProgramError::InvalidArgument)?;

    if parsed_to_pubkey != *to_token_account.key {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let spl_data_seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()];

    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);

    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let token_decimal_mapping =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])?;

    let decimal_mapping = token_decimal_mapping
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

     if (l2_token != decimal_mapping.l2_token.to_string()) {
        return Err(ProgramCustomError::TokenMappingNotFound.into());
    }

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    let forced_withdrawal_messages_buffer = MessagesBuffer::deserialize(
        &mut &forced_withdrawal_messages_buffer_acc.data.borrow()[..],
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = forced_withdrawal_messages_buffer.message_nonce + 1;

    let clock = Clock::get()?;

    let withdraw_info = ForcedWithdrawMessageInfo {
        txn_type:TransactionType::Withdraw,
        nonce: u64_nonce,
        chain_id: CHAIN_ID,
        slot_number: clock.slot,
        from_twine_address: from_twine_address,
        to_l1_pubkey: to_l1_pubkey,
        l1_token: l1_token,
        l2_token: l2_token,
        amount: l2_amount.to_string(),
        data: String::new(), 
    };
    let sign_info = SignMessageInfo {
        nonce: u64_nonce,
        chain_id: CHAIN_ID,
        amount: amount,
        from_twine_address: withdraw_info.from_twine_address.clone(),
        to_l1_pubkey: withdraw_info.to_l1_pubkey.clone(),
        l1_token: withdraw_info.l1_token.clone(),
        l2_token: withdraw_info.l2_token.clone(),
    };

    let recovered_address = recover_address(sign_info.clone(), signature)?;

    if recovered_address.to_lowercase() != withdraw_info.from_twine_address.to_lowercase() {
        return Err(ProgramError::InvalidArgument);
    };

    let payload = TwineChainInstruction::AppendForcedWithdrawalMessage {
        withdraw_info: withdraw_info,
    };

    let mut append_instruction_data = vec![];

    append_instruction_data.extend(payload.try_to_vec().unwrap());

    let append_instruction_accounts = vec![
        AccountMeta::new(*forced_withdrawal_messages_buffer_acc.key, false),
        AccountMeta::new_readonly(*role_manager_acc.key, false),
        AccountMeta::new_readonly(*spl_tokens_vault_data_acc.key, true),
    ];
    let append_instruction = Instruction {
        program_id: *twine_chain_program.key,
        accounts: append_instruction_accounts,
        data: append_instruction_data,
    };

    invoke_signed(
        &append_instruction,
        &[
            forced_withdrawal_messages_buffer_acc.clone(),
            role_manager_acc.clone(),
            spl_tokens_vault_data_acc.clone(),
        ],
        &[&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;
    Ok(())
}

fn validate_accounts(
    user_account: &AccountInfo,
    to_token_account: &AccountInfo,
    spl_tokens_vault_data_acc: &AccountInfo,
    mint: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    forced_withdrawal_messages_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !user_account.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if to_token_account.owner != &TOKEN_PROGRAM_ID {
        msg!("Invalid to_token_account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if spl_tokens_vault_data_acc.owner != program_id {
        msg!("Invalid SPL tokens vault data account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if mint.owner != &TOKEN_PROGRAM_ID {
        msg!("Invalid mint account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_decimal_mappings_acc.owner != program_id {
        msg!("Invalid token decimal mappings account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if forced_withdrawal_messages_buffer_acc.owner != &twine_chain_program_id {
        msg!("Invalid forced withdrawal messages buffer account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if role_manager_acc.owner != &twine_chain_program_id {
        msg!("Invalid role manager account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if twine_chain_program.key != &twine_chain_program_id {
        msg!("Invalid Twine chain program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}
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
use mock_clock::Clock;

// Mock recover_address for tests
#[cfg(test)]
fn recover_address(
    _sign_info: SignMessageInfo,
    _signature: Vec<u8>,
) -> Result<String, ProgramError> {
    Ok("0x1234567890123456789012345678901234567890".to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{TokenDecimalMappingData, TokenDecimalMappings};
    use solana_program::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, system_program};
    use spl_token::ID as TOKEN_PROGRAM_ID;
    use std::str::FromStr;
    use twine_chain::core::state::MessagesBuffer;

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
    fn test_forced_spl_token_withdrawal_success() {
        let program_id = Pubkey::new_unique();
        let twine_chain_programs_id = Pubkey::new_unique();

        // Prepare keys
        let user_key = Pubkey::new_unique();
        let to_token_key = Pubkey::new_unique();
        let (spl_tokens_vault_data_key, _) =
            Pubkey::find_program_address(&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()], &program_id);
        let token_decimal_mappings_key = Pubkey::new_unique();
        let forced_withdrawal_buffer_key = Pubkey::new_unique();
        let role_manager_key = Pubkey::new_unique();

        // Prepare test data
        let token_mappings = TokenDecimalMappings {
            is_initialized: true,
            mappings: vec![TokenDecimalMappingData {
                l1_token: "So11111111111111111111111111111111111111112".to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                l1_decimals: 9,
                l2_decimals: 18,
            }],
        };
        let mut token_mappings_serialized = token_mappings.try_to_vec().unwrap();

        let forced_withdrawal_buffer = MessagesBuffer {
            is_initialized: true,
            message_nonce: 5,
            messages: Vec::new(),
            chain_id: todo!(),
        };
        let mut forced_withdrawal_buffer_serialized =
            forced_withdrawal_buffer.try_to_vec().unwrap();

        // Setup lamports and owners
        let mut user_lamports = 1_000_000_000u64;
        let mut to_token_lamports = 1_000_000u64;
        let mut spl_vault_lamports = 1_000_000u64;
        let mut token_mappings_lamports = 1_000_000u64;
        let mut forced_withdrawal_lamports = 1_000_000u64;
        let mut role_manager_lamports = 1_000_000u64;
        let mut twine_chain_lamports = 0u64;

        let mut user_owner = system_program::id();
        let mut to_token_owner = TOKEN_PROGRAM_ID;
        let mut spl_vault_owner = program_id;
        let mut token_mappings_owner = program_id;
        let mut forced_withdrawal_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut twine_chain_owner = system_program::id();

        // Create dummy data buffers
        let mut spl_vault_data = vec![0; 100];
        let mut role_manager_data = vec![0; 100];
        let mut user_data = vec![0; 100];
        let mut twine_chain_data = vec![0; 100];

        // Create accounts
        let user_account = create_test_account(
            &user_key,
            true,
            false,
            &mut user_lamports,
            &mut user_data,
            &mut user_owner,
        );

        let mut binding = vec![0; 100];
        let to_token_account = create_test_account(
            &to_token_key,
            false,
            false,
            &mut to_token_lamports,
            &mut binding,
            &mut to_token_owner,
        );

        let spl_tokens_vault_data_account = create_test_account(
            &spl_tokens_vault_data_key,
            false,
            true,
            &mut spl_vault_lamports,
            &mut spl_vault_data,
            &mut spl_vault_owner,
        );

        let token_decimal_mappings_account = create_test_account(
            &token_decimal_mappings_key,
            false,
            false,
            &mut token_mappings_lamports,
            &mut token_mappings_serialized,
            &mut token_mappings_owner,
        );

        let forced_withdrawal_buffer_account = create_test_account(
            &forced_withdrawal_buffer_key,
            false,
            true,
            &mut forced_withdrawal_lamports,
            &mut forced_withdrawal_buffer_serialized,
            &mut forced_withdrawal_owner,
        );

        let role_manager_account = create_test_account(
            &role_manager_key,
            false,
            false,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        let twine_chain_program_account = create_test_account(
            &twine_chain_programs_id,
            false,
            false,
            &mut twine_chain_lamports,
            &mut twine_chain_data,
            &mut twine_chain_owner,
        );

        let accounts = vec![
            user_account,
            to_token_account,
            spl_tokens_vault_data_account,
            token_decimal_mappings_account,
            forced_withdrawal_buffer_account,
            role_manager_account,
            twine_chain_program_account,
        ];

        // Test parameters
        let from_twine_address = "0x1234567890123456789012345678901234567890".to_string();
        let to_l1_pubkey = to_token_key.to_string();
        let l1_token = "So11111111111111111111111111111111111111112".to_string();
        let l2_token = "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string();
        let amount = 1000000;
        let signature = vec![1; 65];

        let result = forced_spl_token_withdrawal(
            &program_id,
            &accounts,
            from_twine_address,
            to_l1_pubkey,
            l1_token,
            l2_token,
            amount,
            signature,
        );
        assert!(result.is_ok());
    }
}
