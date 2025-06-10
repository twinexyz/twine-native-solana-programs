use crate::core::error::ProgramCustomError;
use crate::core::state::{SplTokensVaultData, TokenDecimalMappings};
use crate::utils::constants::SPL_TOKENS_VAULT_DATA_PREFIX;
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(test))]
use solana_program::clock::Clock;
use solana_program::pubkey::Pubkey;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    sysvar::Sysvar,
};
use spl_token::instruction as token_instruction;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use twine_chain::core::state::{DepositMessageInfo, DepositMessagesBuffer};
use twine_chain::utils::constants::DEPOSIT_BUFFER_PREFIX;

pub fn spl_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let deposit_messages_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;

    if amount == 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if l1_token != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let spl_data_seeds = &[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()];
    let (spl_data_key, spl_data_bump) = Pubkey::find_program_address(spl_data_seeds, program_id);
    if spl_data_key != *spl_tokens_vault_data_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let user_token_data = TokenAccount::unpack(&user_token_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if user_token_data.amount < amount {
        msg!("User token account has insufficient funds");
        return Err(ProgramCustomError::InsufficientFundsForTransfer.into());
    }

    let token_decimal_mappings =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;
    let decimal_mapping = token_decimal_mappings
        .get_mapping(&l1_token)
        .ok_or(ProgramCustomError::TokenMappingNotFound)?;

    let l2_amount = TokenDecimalMappings::convert_l1_to_l2(
        amount,
        decimal_mapping.l1_decimals,
        decimal_mapping.l2_decimals,
    )
    .map_err(|_| ProgramCustomError::TokenMappingNotFound)?;

    let transfer_instruction = token_instruction::transfer(
        token_program.key,
        user_token_account.key,
        spl_tokens_vault_acc.key,
        user.key,
        &[],
        amount,
    )?;

    invoke(
        &transfer_instruction,
        &[
            user_token_account.clone(),
            spl_tokens_vault_acc.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;

    let mut spl_tokens_vault_data =
        SplTokensVaultData::try_from_slice(&spl_tokens_vault_data_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    spl_tokens_vault_data.update_deposit(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut *spl_tokens_vault_data_acc.data.borrow_mut())
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

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
        from_l1_pubkey: user_token_account.key.to_string(),
        to_twine_address: receiver_twine_address,
        l1_token: l1_token,
        l2_token: l2_token,
        amount: l2_amount,
    };

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
            deposit_messages_buffer_acc.clone(),
            role_manager_acc.clone(),
            spl_tokens_vault_data_acc.clone(),
        ],
        &[&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;

    msg!("SPL token deposit successful");
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
        SplTokensVaultData, TokenDecimalMapping, TokenDecimalMappings, TokenDepositData,
    };
    use solana_program::{account_info::AccountInfo, clock::Epoch,pubkey::Pubkey, system_program};
    use spl_token::state::Account as TokenAccount;
    use twine_chain::core::state::DepositMessagesBuffer;

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

    fn create_token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0u8; TokenAccount::LEN];
        let token_account = TokenAccount {
            mint,
            owner,
            amount,
            delegate: None.into(),
            state: spl_token::state::AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };

        // Pack the token account data
        spl_token::state::Account::pack(token_account, &mut data).unwrap();
        data
    }
    #[test]
    fn test_spl_token_withdrawal_success() {
        let program_id = Pubkey::new_unique();
        let system_program_id = system_program::id();

        // Setup keys
        let user_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let user_token_account_key = Pubkey::new_unique();
        let spl_tokens_vault_key = Pubkey::new_unique();
        let (spl_tokens_vault_data_key, _) =
            Pubkey::find_program_address(&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes()], &program_id);
        let (deposit_buffer_key, _) =
            Pubkey::find_program_address(&[DEPOSIT_BUFFER_PREFIX.as_bytes()], &program_id);
        let token_decimal_mappings_key = Pubkey::new_unique();
        let role_manager_key = Pubkey::new_unique();
        let twine_chain_program_id = Pubkey::new_unique();
        let spl_token_program_id = spl_token::id();

        let mut user_token_account_data = create_token_account_data(mint_key, user_key, 1_000_000);

        let mut spl_tokens_vault_data = SplTokensVaultData {
            is_initialized: true,
            total_deposited_amount: vec![TokenDepositData {
                token_id: mint_key,
                amount: 1_000_000,
            }],
        }
        .try_to_vec()
        .unwrap();

        let mut token_mappings_data = TokenDecimalMappings {
            is_initialized: true,
            mappings: vec![TokenDecimalMapping {
                l1_token: mint_key.to_string(),
                l2_token: "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
                l1_decimals: 6,
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

        // Setup additional data buffers
        let mut vault_data = create_token_account_data(mint_key, spl_tokens_vault_data_key, 0);
        let mut mint_data = vec![0; 100];
        let mut role_manager_data = vec![1; 1000];
        let mut user_data = vec![0; 100];
        let mut twine_chain_data = vec![0; 100];

        // Setup lamports and owners
        let mut user_lamports = 1_000_000u64;
        let mut user_token_account_lamports = 1_000_000u64;
        let mut spl_vault_data_lamports = 1_000_000u64;
        let mut spl_vault_lamports = 1_000_000u64;
        let mut mint_lamports = 1_000_000u64;
        let mut token_program_lamports = 0u64;
        let mut token_mappings_lamports = 1_000_000u64;
        let mut deposit_buffer_lamports = 1_000_000u64;
        let mut role_manager_lamports = 1_000_000u64;
        let mut twine_chain_lamports = 0u64;

        let mut user_owner = system_program::id();
        let mut user_token_account_owner = spl_token_program_id;
        let mut spl_vault_data_owner = program_id;
        let mut spl_vault_owner = spl_token_program_id;
        let mut mint_owner = system_program_id;
        let mut token_program_owner = system_program::id();
        let mut token_mappings_owner = program_id;
        let mut deposit_buffer_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut twine_chain_owner = system_program::id();

        // Create accounts
        let user_account = create_test_account(
            &user_key,
            true,
            false,
            &mut user_lamports,
            &mut user_data,
            &mut user_owner,
        );

        let user_token_account = create_test_account(
            &user_token_account_key,
            false,
            true,
            &mut user_token_account_lamports,
            &mut user_token_account_data,
            &mut user_token_account_owner,
        );

        let spl_tokens_vault_data_account = create_test_account(
            &spl_tokens_vault_data_key,
            false,
            true,
            &mut spl_vault_data_lamports,
            &mut spl_tokens_vault_data,
            &mut spl_vault_data_owner,
        );

        let spl_tokens_vault_account = create_test_account(
            &spl_tokens_vault_key,
            false,
            true,
            &mut spl_vault_lamports,
            &mut vault_data,
            &mut spl_vault_owner,
        );

        let mint_account = create_test_account(
            &mint_key,
            false,
            false,
            &mut mint_lamports,
            &mut mint_data,
            &mut mint_owner,
        );

        let token_program_account = create_test_account(
            &spl_token_program_id,
            false,
            false,
            &mut token_program_lamports,
            &mut [],
            &mut token_program_owner,
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

        let role_manager_account = create_test_account(
            &role_manager_key,
            false,
            false,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
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
            user_token_account,
            spl_tokens_vault_data_account,
            spl_tokens_vault_account,
            mint_account,
            token_program_account,
            token_decimal_mappings_account,
            deposit_buffer_account,
            role_manager_account,
            twine_chain_program_account,
        ];
        let result = spl_token_deposit(
            &program_id,
            &accounts,
            "0x1234567890123456789012345678901234567890".to_string(),
            mint_key.to_string(),
            "0xa345a01f6C6c1E51E1B2C5f576FBF20B34DadB88".to_string(),
            500_000,
        );
        assert!(result.is_ok());
    }
}
