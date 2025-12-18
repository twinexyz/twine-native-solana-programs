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
        SendMsgParams, Store, ENDPOINT_SEED, ENDPOINT_SEND_DISCRIMINATOR, MESSAGE_LIB_SEED,
        NONCE_SEED, SEND_LIBRARY_CONFIG_SEED, STORE_SEED,
    },
};

pub fn send_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: SendMsgParams,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let store_acc = next_account_info(account_info_iter)?;
    let send_library_program_acc = next_account_info(account_info_iter)?;
    let send_library_config_acc = next_account_info(account_info_iter)?;
    let default_send_library_config_acc = next_account_info(account_info_iter)?;
    let send_library_info_acc = next_account_info(account_info_iter)?;
    let endpoint_acc = next_account_info(account_info_iter)?;
    let nonce_acc = next_account_info(account_info_iter)?;
    let endpoint_event_authority_acc = next_account_info(account_info_iter)?;
    let endpoint_program_acc = next_account_info(account_info_iter)?;

    let store_bump = validate_accounts(program_id, store_acc)?;

    // Accounts required by library send
    let uln_acc = next_account_info(account_info_iter)?;
    let send_config_acc = next_account_info(account_info_iter)?;
    let default_send_config_acc = next_account_info(account_info_iter)?;
    let payer_acc = next_account_info(account_info_iter)?;
    let treasury_acc = next_account_info(account_info_iter)?;
    let system_program_acc = next_account_info(account_info_iter)?;
    let library_event_authority_acc = next_account_info(account_info_iter)?;
    let library_program = next_account_info(account_info_iter)?;

    // Remaining accounts for dvn and executor
    let executor_program_acc = next_account_info(account_info_iter)?;
    let executor_config_acc = next_account_info(account_info_iter)?;
    let price_feed_executor_acc = next_account_info(account_info_iter)?;
    let price_feed_config_executor_acc = next_account_info(account_info_iter)?;

    let dvn_program_acc = next_account_info(account_info_iter)?;
    let dvn_config_acc = next_account_info(account_info_iter)?;
    let price_feed_dvn_acc = next_account_info(account_info_iter)?;
    let price_feed_config_dvn_acc = next_account_info(account_info_iter)?;

    // --- Construct instruction data for Endpoint::send ---
    let discriminator = ENDPOINT_SEND_DISCRIMINATOR;
    let params_data = params.try_to_vec()?;

    let mut send_data = Vec::with_capacity(8 + params_data.len());
    send_data.extend_from_slice(&discriminator);
    send_data.extend_from_slice(&params_data);

    // ^ 8-byte discriminator for "global:send" (computed via SHA-256)

    // --- Construct account metas for CPI ---
    let mut cpi_accounts = vec![];
    cpi_accounts.push(AccountMeta::new(*store_acc.key, true));
    cpi_accounts.push(AccountMeta::new_readonly(
        *send_library_program_acc.key,
        false,
    ));
    cpi_accounts.push(AccountMeta::new(*send_library_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(
        *default_send_library_config_acc.key,
        false,
    ));
    cpi_accounts.push(AccountMeta::new_readonly(*send_library_info_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*endpoint_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*nonce_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*endpoint_event_authority_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*endpoint_program_acc.key, false));

    cpi_accounts.push(AccountMeta::new(*uln_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*send_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*default_send_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*payer_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*treasury_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*system_program_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*library_event_authority_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*library_program.key, false));

    cpi_accounts.push(AccountMeta::new_readonly(*executor_program_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*executor_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        *price_feed_executor_acc.key,
        false,
    ));
    cpi_accounts.push(AccountMeta::new_readonly(
        *price_feed_config_executor_acc.key,
        false,
    ));

    cpi_accounts.push(AccountMeta::new_readonly(*dvn_program_acc.key, false));
    cpi_accounts.push(AccountMeta::new(*dvn_config_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*price_feed_dvn_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        *price_feed_config_dvn_acc.key,
        false,
    ));

    // Build the CPI instruction for the Endpoint program's `send`
    let send_ix = Instruction {
        program_id: *endpoint_program_acc.key,
        accounts: cpi_accounts,
        data: send_data,
    };

    // Invoke the Endpoint::send instruction. The Store PDA signs via invoke_signed.
    invoke_signed(
        &send_ix,
        &[
            store_acc.clone(),
            send_library_program_acc.clone(),
            send_library_config_acc.clone(),
            default_send_library_config_acc.clone(),
            send_library_info_acc.clone(),
            endpoint_acc.clone(),
            nonce_acc.clone(),
            endpoint_event_authority_acc.clone(),
            endpoint_program_acc.clone(),
            uln_acc.clone(),
            send_config_acc.clone(),
            default_send_config_acc.clone(),
            payer_acc.clone(),
            treasury_acc.clone(),
            system_program_acc.clone(),
            library_event_authority_acc.clone(),
            library_program.clone(),
            executor_program_acc.clone(),
            executor_config_acc.clone(),
            price_feed_executor_acc.clone(),
            price_feed_config_executor_acc.clone(),
            dvn_program_acc.clone(),
            dvn_config_acc.clone(),
            price_feed_dvn_acc.clone(),
            price_feed_config_dvn_acc.clone(),
        ],
        &[&[STORE_SEED, &[store_bump]]], // seeds for Store PDA signer
    )?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    store_acc: &AccountInfo,
    // send_library_acc: &AccountInfo,
    // send_library_config_acc: &AccountInfo,
    // default_send_library_config_acc: &AccountInfo,
    // send_library_info_acc: &AccountInfo,
    // endpoint_acc: &AccountInfo,
    // nonce_acc: &AccountInfo,
    // params: SendMsgParams,
) -> Result<u8, ProgramError> {
    // Validate Store account
    let expected_store_key = Pubkey::find_program_address(&[STORE_SEED], program_id).0;
    if store_acc.key != &expected_store_key {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let store_data = Store::try_from_slice(&store_acc.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // let endpoint_program_id = store_data.endpoint_program;
    let store_bump = store_data.bump;

    // // Validate Send Library Config Account
    // let expected_send_library_acc = Pubkey::find_program_address(
    //     &[
    //         SEND_LIBRARY_CONFIG_SEED,
    //         expected_store_key.as_ref(),
    //         &params.dst_eid.to_be_bytes(),
    //     ],
    //     &endpoint_program_id,
    // )
    // .0;
    // if send_library_config_acc.key != &expected_send_library_acc {
    //     return Err(ProgramCustomError::InvalidPDA.into());
    // }

    // // Validate Default Send Library Config Account
    // let expected_default_send_library_config_acc = Pubkey::find_program_address(
    //     &[SEND_LIBRARY_CONFIG_SEED, &params.dst_eid.to_be_bytes()],
    //     &endpoint_program_id,
    // )
    // .0;
    // if default_send_library_config_acc.key != &expected_default_send_library_config_acc {
    //     return Err(ProgramCustomError::InvalidPDA.into());
    // }

    // // Validate Send Library Info Account
    // let expected_send_library_info_acc = Pubkey::find_program_address(
    //     &[MESSAGE_LIB_SEED, &send_library_acc.key.to_bytes()],
    //     &endpoint_program_id,
    // )
    // .0;
    // if send_library_info_acc.key != &expected_send_library_info_acc {
    //     return Err(ProgramCustomError::InvalidPDA.into());
    // }

    // // Validate Endpoint account
    // let expected_endpoint_acc =
    //     Pubkey::find_program_address(&[ENDPOINT_SEED], &endpoint_program_id).0;
    // if endpoint_acc.key != &expected_endpoint_acc {
    //     return Err(ProgramCustomError::InvalidPDA.into());
    // }

    // // Validate Nonce Account
    // let expected_nonce_acc = Pubkey::find_program_address(
    //     &[
    //         NONCE_SEED,
    //         &expected_store_key.to_bytes(),
    //         &params.dst_eid.to_be_bytes(),
    //         &params.receiver[..],
    //     ],
    //     &endpoint_program_id,
    // )
    // .0;
    // if nonce_acc.key != &expected_nonce_acc {
    //     return Err(ProgramCustomError::InvalidPDA.into());
    // }

    Ok(store_bump)
}
