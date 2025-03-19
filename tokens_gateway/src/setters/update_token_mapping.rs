use crate::core::error::ProgramCustomError;
use crate::core::state::{RoleType, TokenDecimalMappings, TokensGatewayRoleManager};
use crate::utils::ethereum_checks::is_valid_ethereum_address;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn update_token_mapping(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let authority_acc = next_account_info(account_info_iter)?;
    let token_decimal_mappings_acc = next_account_info(account_info_iter)?;
    let role_manager_acc = next_account_info(account_info_iter);

    // Verify role
    let role_manager =
        TokensGatewayRoleManager::try_from_slice(&role_manager_acc.unwrap().data.borrow())?;
    if !role_manager.has_role(&authority_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }
    // Verify admin privileges
    if !authority_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate token addresses
    if l1_token.is_empty() {
        return Err(ProgramCustomError::InvalidL1Token.into());
    }

    if l2_token.is_empty() {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }

    if token_decimal_mappings_acc.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !is_valid_ethereum_address(&l2_token)? {
        return Err(ProgramCustomError::InvalidL2Token.into());
    }
    let mut token_decimal_mappings =
        TokenDecimalMappings::try_from_slice(&token_decimal_mappings_acc.data.borrow())?;
    token_decimal_mappings.update_mapping(l1_token, l2_token, l1_decimals, l2_decimals)?;

    token_decimal_mappings
        .serialize(&mut *token_decimal_mappings_acc.data.borrow_mut())
        .map_err(|_| ProgramError::AccountDataTooSmall)?;

    msg!("Token Mapping Updated");
    Ok(())
}
