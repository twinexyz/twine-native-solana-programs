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
        SetConfigParams, Store, ENDPOINT_SET_CONFIG_DISCEIMINATOR, MESSAGE_LIB_SEED, OAPP_SEED,
        STORE_SEED,
    },
};

pub fn set_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: SetConfigParams,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let store_acc = next_account_info(account_info_iter)?;
    let oapp_registry_acc = next_account_info(account_info_iter)?;
    let message_lib_info_acc = next_account_info(account_info_iter)?;
    let message_lib_acc = next_account_info(account_info_iter)?;
    let message_lib_program_acc = next_account_info(account_info_iter)?;
    let uln_acc = next_account_info(account_info_iter)?;
    let send_config_acc = next_account_info(account_info_iter)?;
    let receive_config_acc = next_account_info(account_info_iter)?;
    let default_send_config_acc = next_account_info(account_info_iter)?;
    let default_receive_config_acc = next_account_info(account_info_iter)?;
    let event_acc = next_account_info(account_info_iter)?;
    let uln_program = next_account_info(account_info_iter)?;
    let endpoint_program = next_account_info(account_info_iter)?;

    let (endpoint_program_id, store_bump) = validate_accounts(
        program_id,
        store_acc,
        oapp_registry_acc,
        message_lib_info_acc,
        message_lib_acc,
        message_lib_program_acc,
        params.clone(),
    )?;

    let discriminator = ENDPOINT_SET_CONFIG_DISCEIMINATOR;
    let params_data = params.try_to_vec()?;

    let mut send_data = Vec::with_capacity(8 + params_data.len());
    send_data.extend_from_slice(&discriminator);
    send_data.extend_from_slice(&params_data);

    let mut cpi_accounts = vec![];
    cpi_accounts.push(AccountMeta::new(*store_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*oapp_registry_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*message_lib_info_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*message_lib_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*message_lib_program_acc.key, false));

    cpi_accounts.push(AccountMeta::new(*uln_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*send_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*receive_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*default_send_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*default_receive_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*event_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*uln_program.key, false));

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
            message_lib_info_acc.clone(),
            message_lib_acc.clone(),
            message_lib_program_acc.clone(),
            uln_acc.clone(),
            send_config_acc.clone(),
            receive_config_acc.clone(),
            default_send_config_acc.clone(),
            default_receive_config_acc.clone(),
            event_acc.clone(),
            uln_program.clone(),
            endpoint_program.clone(),
        ],
        &[&[STORE_SEED, &[store_bump]]], // seeds for Store PDA signer
    )?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    store_acc: &AccountInfo,
    oapp_registry_acc: &AccountInfo,
    message_lib_info_acc: &AccountInfo,
    message_lib_acc: &AccountInfo,
    message_lib_program_acc: &AccountInfo,
    params: SetConfigParams,
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
        Pubkey::find_program_address(&[OAPP_SEED, params.oapp.as_ref()], &endpoint_program_id).0;

    if oapp_registry_acc.key != &expected_oapp_registry_acc {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate Message library
    let expected_message_library_info_acc = Pubkey::find_program_address(
        &[MESSAGE_LIB_SEED, &message_lib_acc.key.to_bytes()],
        &endpoint_program_id,
    )
    .0;

    if message_lib_info_acc.key != &expected_message_library_info_acc {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate Message Library
    let expected_message_lib_acc =
        Pubkey::find_program_address(&[MESSAGE_LIB_SEED], message_lib_program_acc.key).0;

    if message_lib_acc.key != &expected_message_lib_acc {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    Ok((endpoint_program_id, store_bump))
}
