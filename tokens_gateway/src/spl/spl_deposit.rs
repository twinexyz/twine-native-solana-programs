use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_token::{
    instruction as token_instruction, solana_program::program_pack::Pack,
    state::Account as TokenAccount,
};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{DetailedMessagesBuffer, MessageInfo, TransactionType},
    },
    utils::constants::{FORCED_WITHDRAW_MESSAGE_TYPE, MESSAGES_BUFFER_PREFIX},
    ID as twine_chain_program_id,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{SplTokensVaultData, TokenDecimalMappings},
    },
    utils::{
        constants::{DEPOSIT_TRANSACTION, SPL_TOKENS_VAULT_DATA_PREFIX},
        ethereum_checks::is_valid_ethereum_address,
    },
};

pub fn spl_token_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramCustomError::InvalidAmount.into());
    }
    if l1_token == "11111111111111111111111111111111" {
        return Err(ProgramCustomError::InvalidToken.into());
    }

    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if !is_valid_ethereum_address(&receiver_twine_address)? {
        return Err(ProgramCustomError::InvalidReceiver.into());
    }

    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let spl_tokens_vault_data_acc = next_account_info(account_info_iter)?;
    let spl_tokens_vault_acc = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let messages_buffer_acc = next_account_info(account_info_iter)?;
    let detailed_messages_buffer_acc = next_account_info(account_info_iter)?;
    let twine_chain_role_manager_acc = next_account_info(account_info_iter)?;
    let twine_chain_program = next_account_info(account_info_iter)?;
    if l1_token != mint.key.to_string() {
        return Err(ProgramCustomError::InvalidToken.into());
    }
    validate_accounts(
        user,
        user_token_account,
        spl_tokens_vault_data_acc,
        spl_tokens_vault_acc,
        mint,
        token_program,
        token_decimal_mappings_acc,
        messages_buffer_acc,
        detailed_messages_buffer_acc,
        twine_chain_role_manager_acc,
        twine_chain_program,
        program_id,
    );
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
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])?;

    let decimal_mapping = token_decimal_mappings
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
    let transfer_instruction = token_instruction::transfer(
        &spl_token::id(),
        &user_token_account.key,
        &spl_tokens_vault_acc.key,
        &user.key,
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
        SplTokensVaultData::deserialize(&mut &spl_tokens_vault_data_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    spl_tokens_vault_data.update_deposit(*mint.key, amount)?;

    spl_tokens_vault_data
        .serialize(&mut &mut spl_tokens_vault_data_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    let (expected_message_pda, _) = Pubkey::find_program_address(
        &[MESSAGES_BUFFER_PREFIX.as_bytes()],
        &twine_chain_program_id,
    );

    if expected_message_pda != *messages_buffer_acc.key {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let deposit_message_buffer =
        DetailedMessagesBuffer::deserialize(&mut &detailed_messages_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let u64_nonce = deposit_message_buffer.message_nonce + 1;

    let clock = Clock::get()?;

    let deposit_info = MessageInfo {
        txn_type: TransactionType::Deposit,
        nonce: u64_nonce,
        chain_id: 900,
        slot_number: clock.slot,
        l1_pubkey: user_token_account.key.to_string(),
        twine_address: receiver_twine_address,
        l1_token: l1_token,
        l2_token: l2_token,
        amount: l2_amount,
        data: data,
    };

    let payload = TwineChainInstruction::AppendDepositMessage {
        deposit_info: deposit_info,
    };

    let mut append_instruction_data = vec![];

    append_instruction_data.extend(payload.try_to_vec().unwrap());

    let append_instruction_accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(*messages_buffer_acc.key, false),
        AccountMeta::new(*detailed_messages_buffer_acc.key, false),
        AccountMeta::new_readonly(*twine_chain_role_manager_acc.key, false),
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
            messages_buffer_acc.clone(),
            detailed_messages_buffer_acc.clone(),
            twine_chain_role_manager_acc.clone(),
            spl_tokens_vault_data_acc.clone(),
        ],
        &[&[SPL_TOKENS_VAULT_DATA_PREFIX.as_bytes(), &[spl_data_bump]]],
    )?;

    msg!("SPL token deposit successful");

    Ok(())
}

fn validate_accounts(
    user: &AccountInfo,
    user_token_account: &AccountInfo,
    spl_tokens_vault_data_acc: &AccountInfo,
    spl_tokens_vault_acc: &AccountInfo,
    mint: &AccountInfo,
    token_program: &AccountInfo,
    token_decimal_mappings_acc: &AccountInfo,
    messages_buffer_acc: &AccountInfo,
    detailed_messages_buffer_acc: &AccountInfo,
    twine_chain_role_manager_acc: &AccountInfo,
    twine_chain_program: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !user.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if user_token_account.owner != &spl_token::id() {
        msg!("Invalid user token account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if spl_tokens_vault_data_acc.owner != program_id {
        msg!("Invalid SPL tokens vault data account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if spl_tokens_vault_acc.owner != &spl_token::id() {
        msg!("Invalid SPL tokens vault account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if mint.owner != &spl_token::id() {
        msg!("Invalid mint account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_program.key != &spl_token::id() {
        msg!("Invalid token program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_decimal_mappings_acc.owner != program_id {
        msg!("Invalid token decimal mappings account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if messages_buffer_acc.owner != &twine_chain_program_id {
        msg!("Invalid messages buffer account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if detailed_messages_buffer_acc.owner != &twine_chain_program_id {
        msg!("Invalid detailed messages buffer account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    if twine_chain_role_manager_acc.owner != &twine_chain_program_id {
        msg!("Invalid role manager account owner");
        return Err(ProgramError::IncorrectProgramId);
    }
    if twine_chain_program.key != &twine_chain_program_id {
        msg!("Invalid Twine chain program account");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}
