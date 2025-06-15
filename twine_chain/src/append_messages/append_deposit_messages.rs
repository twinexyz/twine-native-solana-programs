use crate::core::error::ProgramCustomError;
use crate::core::state::{
    DepositMessageInfo, DepositMessagesBuffer, RoleType, TwineChainRoleManager,
};
use crate::utils::address_derivation::{
    derive_deposit_message_buffer, derive_role_manager, verify_derived_address,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn append_deposit_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_info: DepositMessageInfo,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let deposit_message_buffer_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter)?;
    let initializer_acc = next_account_info(account_info_iter)?;

    validate_accounts(
        program_id,
        deposit_message_buffer_acc,
        role_manager_acc,
        initializer_acc,
    )?;

    // Validate data length
    let total_len = 8
        + 8
        + 8
        + (4 + deposit_info.from_l1_pubkey.len())
        + (4 + deposit_info.to_twine_address.len())
        + (4 + deposit_info.l1_token.len())
        + (4 + deposit_info.l2_token.len())
        + (4 + deposit_info.amount.len());

    if total_len > DepositMessageInfo::LEN {
        return Err(ProgramCustomError::InvalidDataLength.into());
    }

    // Deserialize account data
    let mut deposits =
        DepositMessagesBuffer::deserialize(&mut &deposit_message_buffer_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if deposit message buffer is initialized
    if !deposits.is_initialized() {
        return Err(ProgramCustomError::UninitializedAccount.into());
    }

    // Update Deposits
    deposits.deposit_messages.push(deposit_info.clone());
    deposits.deposit_nonce += 1;

    deposits
        .serialize(&mut &mut deposit_message_buffer_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!(
        "event=DepositSuccessful nonce={} from_l1_pubkey={} to_twine_address={} l1_token={} l2_token={} chain_id={} amount={} slot_number={}",
        deposit_info.nonce,
        deposit_info.from_l1_pubkey,
        deposit_info.to_twine_address,
        deposit_info.l1_token,
        deposit_info.l2_token,
        deposit_info.chain_id,
        deposit_info.amount,
        deposit_info.slot_number
    );
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    deposit_message_buffer_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    initializer_acc: &AccountInfo,
) -> ProgramResult {
    // Validate signer
    if !initializer_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_deposit_pda, _) = derive_deposit_message_buffer(program_id);
    verify_derived_address(expected_deposit_pda, deposit_message_buffer_acc)?;

    let (expected_role_manager_pda, _) = derive_role_manager(program_id);
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
    use crate::utils::constants::{INITIAL_CHAIN_ADMIN, MAX_QUEUE_SIZE, MAX_ROLES};
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
    fn test_append_deposit_messages() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::new_unique();

        // Get the required accounts
        let (deposit_message_buffer_key, _) = derive_deposit_message_buffer(&program_id);
        let (role_manager_key, _) = derive_role_manager(&program_id);
        let initializer_key = Pubkey::from_str(INITIAL_CHAIN_ADMIN)?;
        let system_program_id = system_program::id();

        // Required space for each account
        let deposit_message_buffer_space: usize =
            1 + 8 + 4 + (MAX_QUEUE_SIZE * DepositMessageInfo::LEN);
        let role_manager_space: usize = 1 + 32 + 32 + 32 + (4 + MAX_ROLES * 33);

        // Setup Account Lamports
        let rent = Rent::default();
        let mut deposit_message_buffer_lamports =
            rent.minimum_balance(deposit_message_buffer_space);
        let mut role_manager_lamports = rent.minimum_balance(role_manager_space);
        let mut initializer_lamports = 1_000_000_000;

        // Setup Account Data
        let buffer = DepositMessagesBuffer {
            is_initialized: true,
            deposit_nonce: 0,
            deposit_messages: vec![],
        };

        let mut deposit_buffer_data = vec![0u8; deposit_message_buffer_space];

        let mut temp = vec![];
        buffer.serialize(&mut temp)?;

        deposit_buffer_data[..temp.len()].copy_from_slice(&temp);

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
        let mut deposit_message_buffer_owner = program_id;
        let mut role_manager_owner = program_id;
        let mut initializer_owner = system_program_id;

        // Create required account infos
        let deposit_message_buffer_account = create_test_account_info(
            &deposit_message_buffer_key,
            false,
            true,
            &mut deposit_message_buffer_lamports,
            &mut deposit_buffer_data,
            &mut deposit_message_buffer_owner,
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
            deposit_message_buffer_account.clone(),
            role_manager_account.clone(),
            initializer_account.clone(),
        ];

        // call the set function
        let dummy_msg = DepositMessageInfo {
            nonce: 1,
            chain_id: 100,
            slot_number: 200,
            from_l1_pubkey: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            to_twine_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            l1_token: "6gEHwA9cX51JCMoQQnS78Y3FfX6fwCr4urAY2BQJkNvf".to_string(),
            l2_token: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            amount: "1000000000000000000".to_string(),
        };

        let result = append_deposit_message(&program_id, &accounts, dummy_msg);
        assert!(result.is_ok(), "Deposit failed: {:?}", result.err());

        // verify setter
        let deposit_buffer_data = DepositMessagesBuffer::deserialize(
            &mut &deposit_message_buffer_account.data.borrow()[..],
        )?;

        assert_eq!(
            deposit_buffer_data.deposit_messages.len(),
            1,
            "Value should be set to value provided"
        );
        Ok(())
    }
}
