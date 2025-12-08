use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::core::{
    error::ProgramCustomError,
    state::{
        SetSendLibraryParams, Store, ENDPOINT_SET_SEND_LIB_DISCRIMINATOR, MESSAGE_LIB_SEED,
        OAPP_SEED, SEND_LIBRARY_CONFIG_SEED, STORE_SEED,
    },
};

pub fn set_send_library(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: SetSendLibraryParams,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let store_acc = next_account_info(account_info_iter)?;
    let oapp_registry_acc = next_account_info(account_info_iter)?;
    let send_library_config_acc = next_account_info(account_info_iter)?;
    let message_library_info_acc = next_account_info(account_info_iter)?;
    let event_authority_acc = next_account_info(account_info_iter)?;
    let endpoint_program_acc = next_account_info(account_info_iter)?;

    let (endpoint_program_id, store_bump) = validate_accounts(
        program_id,
        store_acc,
        oapp_registry_acc,
        send_library_config_acc,
        message_library_info_acc,
        params.clone(),
    )?;

    let discriminator = ENDPOINT_SET_SEND_LIB_DISCRIMINATOR;
    let params_data = params.try_to_vec()?;

    let mut send_data = Vec::with_capacity(8 + params_data.len());
    send_data.extend_from_slice(&discriminator);
    send_data.extend_from_slice(&params_data);

    let mut cpi_accounts = vec![];
    cpi_accounts.push(AccountMeta::new(*store_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*oapp_registry_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*send_library_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*message_library_info_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*event_authority_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*endpoint_program_acc.key, false));

    let send_ix = Instruction {
        program_id: endpoint_program_id,
        accounts: cpi_accounts,
        data: send_data,
    };

    invoke_signed(
        &send_ix,
        &[
            store_acc.clone(),
            oapp_registry_acc.clone(),
            send_library_config_acc.clone(),
            message_library_info_acc.clone(),
            event_authority_acc.clone(),
            endpoint_program_acc.clone(),
        ],
        &[&[STORE_SEED, &[store_bump]]],
    )?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    store_acc: &AccountInfo,
    oapp_registry_acc: &AccountInfo,
    send_library_config_acc: &AccountInfo,
    message_library_info_acc: &AccountInfo,
    params: SetSendLibraryParams,
) -> Result<(Pubkey, u8), ProgramError> {
    // Validate Store account
    let expected_store_key = Pubkey::find_program_address(&[STORE_SEED], program_id).0;
    if store_acc.key != &expected_store_key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let store_data = Store::try_from_slice(&store_acc.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let endpoint_program_id = store_data.endpoint_program;
    let store_bump = store_data.bump;

    // Validate Oapp Registry Account
    let expected_oapp_registry_acc =
        Pubkey::find_program_address(&[OAPP_SEED, params.sender.as_ref()], &endpoint_program_id).0;

    if oapp_registry_acc.key != &expected_oapp_registry_acc {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate Send Library Config Accounts
    let expected_send_library_acc = Pubkey::find_program_address(
        &[
            SEND_LIBRARY_CONFIG_SEED,
            params.sender.as_ref(),
            &params.eid.to_be_bytes(),
        ],
        &endpoint_program_id,
    )
    .0;
    if send_library_config_acc.key != &expected_send_library_acc {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate Message Library Config Account
    let expected_message_library_info_acc = Pubkey::find_program_address(
        &[MESSAGE_LIB_SEED, &params.new_lib.to_bytes()],
        &endpoint_program_id,
    )
    .0;

    if message_library_info_acc.key != &expected_message_library_info_acc {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    Ok((endpoint_program_id, store_bump))
}
