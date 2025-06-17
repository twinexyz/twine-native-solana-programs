use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{RoleType, TokenDecimalMappings, TokensGatewayRoleManager},
    },
    utils::ethereum_checks::is_valid_ethereum_address,
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
    let role_manager_acc = next_account_info(account_info_iter)?;

    let role_manager_data =
        TokensGatewayRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if !role_manager_data.has_role(&authority_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    if !role_manager_data.has_role(authority_acc.key, RoleType::TwineOperationHandler) {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    if !authority_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

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

    let mut token_decimal_mappings_data =
        TokenDecimalMappings::deserialize(&mut &token_decimal_mappings_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    token_decimal_mappings_data.update_mapping(&l1_token, &l2_token, l1_decimals, l2_decimals)?;

    token_decimal_mappings_data
        .serialize(&mut &mut token_decimal_mappings_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!(
        "EVENT:TokenMappingUpdated: l1_token={}, l2_token={}, l1_decimals={}, l2_decimals={}",
        l1_token,
        l2_token,
        l1_decimals,
        l2_decimals
    );

    Ok(())
}
