use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{ExecutionMessageBuffer, RoleType, TwineChainRoleManager},
    },
    utils::address_derivation::{
        derive_execution_message_buffer, 
        derive_role_manager, 
        verify_derived_address,
    },
};


pub fn remove_withdrawal_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nonce: u64,
) -> ProgramResult {
    println!("Inisde Remove withdraw function");
    let account_iter = &mut accounts.iter();
    let execution_message_buffer_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let initializer_acc = next_account_info(account_iter)?;
    println!("Before account validation");

    validate_accounts(
        program_id,
        execution_message_buffer_acc,
        role_manager_acc,
        initializer_acc,
    )?;
    println!("After account validation");

    let mut execution_buffer_data =
        ExecutionMessageBuffer::deserialize(&mut &execution_message_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    println!("Deserialization successful");

    if !execution_buffer_data.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }
    println!("Is initialized case passed");
    println!("Execution buffer data: {:?}", execution_buffer_data.withdrawals);

    let index = execution_buffer_data
        .withdrawals
        .iter()
        .position(|msg| msg.nonce == nonce)
        .ok_or(ProgramCustomError::NonceNotFound)?;
    println!("Removing..");

    // Remove the withdrawal message at the specified index
    execution_buffer_data.withdrawals.remove(index);

    execution_buffer_data
        .serialize(&mut &mut execution_message_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    println!("Passed..");

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    execution_message_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_execution_pda, _execution_bump_seed) =
        derive_execution_message_buffer(program_id);
    verify_derived_address(expected_execution_pda, execution_message_buffer_acc)?;

    let (expected_role_manager_pda, _role_manager_bump_seed) = derive_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    // Checks if signer has required role(MessageAppender)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(initializer_acc.key, RoleType::MessageAppender) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::{core::state::ForcedWithdrawMessageInfo, utils::constants::{INITIAL_CHAIN_ADMIN, MAX_QUEUE_SIZE, MAX_ROLES}};
    use solana_program::{clock::Epoch, rent::Rent, system_program};
    use std::str::FromStr;

    fn create_test_account_info<'a>(
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
    fn test_remove_withdrawal_message() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (execution_message_buffer_key, _) = derive_execution_message_buffer(&program_id);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let initializer_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let execution_message_buffer_space: usize =
            1 + 4 + (MAX_QUEUE_SIZE * ForcedWithdrawMessageInfo::LEN);
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut execution_message_buffer_lamports =
            rent.minimum_balance(execution_message_buffer_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut initializer_lamports = 1_000_000_000;

        // Setup Account Data
        let dummy_msg = ForcedWithdrawMessageInfo {
            nonce: 1,
            chain_id: 100,
            slot_number: 200,
            to_l1_pubkey: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            from_twine_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            l1_token: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            l2_token: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            amount: "1000000000000000000".to_string(),
        };

        let buffer = ExecutionMessageBuffer {
            is_initialized: true,
            withdrawals: vec![dummy_msg],
        };

        let mut execution_buffer_data = vec![0u8; execution_message_buffer_space];

        let mut temp = vec![];
        buffer.serialize(&mut temp)?;

        execution_buffer_data[..temp.len()].copy_from_slice(&temp);

        // Gives role TwineOperationHandler to InitialChainAdmin
        let role_manager_dummy_data = TwineChainRoleManager {
            is_initialized: true,
            chain_admin: Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
            twine_operator: Pubkey::default(),
            token_gateway_program: Pubkey::default(),
            roles: vec![(
                Pubkey::from_str(INITIAL_CHAIN_ADMIN)?,
                RoleType::MessageAppender,
            )],
        };
        let mut role_manager_data = vec![];
        role_manager_dummy_data.serialize(&mut role_manager_data)?;

        let mut initializer_data = vec![];

        // Setup owners
        let mut execution_message_buffer_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut initializer_owner = system_program_id;

        // Create required account infos
        let execution_message_buffer_account = create_test_account_info(
            &execution_message_buffer_key,
            false,
            true,
            &mut execution_message_buffer_lamports,
            &mut execution_buffer_data,
            &mut execution_message_buffer_owner,
        );

        let role_manager_account = create_test_account_info(
            &role_manager_key,
            false,
            true,
            &mut role_manager_lamports,
            &mut role_manager_data,
            &mut role_manager_owner,
        );

        let initializer_account = create_test_account_info(
            &initializer_key,
            true,
            false,
            &mut initializer_lamports,
            &mut initializer_data,
            &mut initializer_owner,
        );

        // Create accounts array in the correct order matching the function
        let accounts = vec![
            execution_message_buffer_account.clone(),
            role_manager_account.clone(),
            initializer_account.clone(),
        ];

        let execution_buffer_data_before_removal = ExecutionMessageBuffer::deserialize(
            &mut &execution_message_buffer_account.data.borrow()[..],
        )?;

        assert_eq!(
            execution_buffer_data_before_removal.withdrawals.len(),
            1,
            "There should be one message"
        );

       
        let result = remove_withdrawal_message(&program_id, &accounts, 1);
        assert!(result.is_ok(), "Setter failed: {:?}", result.err());

        // verify setter
        let execution_buffer_data = ExecutionMessageBuffer::deserialize(
            &mut &execution_message_buffer_account.data.borrow()[..],
        )?;

        assert_eq!(
            execution_buffer_data.withdrawals.len(),
            0,
            "Data should be cleared"
        );
        Ok(())
    }
}
