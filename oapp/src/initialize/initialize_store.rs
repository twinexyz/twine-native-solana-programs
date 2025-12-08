use crate::core::{
    error::ProgramCustomError,
    state::{
        ENDPOINT_REGISTER_OAPP_DISCRIMINATOR, EVENT_SEED, InitStoreParams, OAPP_SEED, RegisterOAppParams, STORE_SEED, Store
    },
};
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

pub fn initalize_store(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: InitStoreParams,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let admin_acc = next_account_info(account_info_iter)?;
    let store_acc = next_account_info(account_info_iter)?;
    let system_program_acc = next_account_info(account_info_iter)?;
    let event_authority_acc = next_account_info(account_info_iter)?;
    let registration_account_acc = next_account_info(account_info_iter)?;
    let endpoint_program_acc = next_account_info(account_info_iter)?;

    // ----------------- Validate Accounts -----------------
    let store_bump = validate_accounts(
        program_id,
        admin_acc,
        store_acc,
        system_program_acc,
        event_authority_acc,
        registration_account_acc,
        endpoint_program_acc,
    )?;

    // ----------------- Create Store PDA -----------------
    let rent = Rent::default();
    let store_space = Store::LEN;
    let required_lamports = rent.minimum_balance(store_space);

    let create_ix = system_instruction::create_account(
        admin_acc.key,
        store_acc.key,
        required_lamports,
        store_space as u64,
        program_id,
    );
    invoke_signed(
        &create_ix,
        &[
            admin_acc.clone(),
            store_acc.clone(),
            system_program_acc.clone(),
        ],
        &[&[STORE_SEED, &[store_bump]]],
    )?;

    let store_data = Store {
        admin: *admin_acc.key,
        endpoint_program: params.endpoint_id,
        bump: store_bump,
        last_msg_size: 0,
        last_msg: [0u8; 256],
    };

    store_data
        .serialize(&mut &mut store_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    // ----------------- Register the OApp with LayerZero Endpoint via CPI -----------------

    let endpoint_program_id = params.endpoint_id;

    // Prepare the instruction data for Endpoint::register_oapp.
   
    let register_oapp_discriminator = ENDPOINT_REGISTER_OAPP_DISCRIMINATOR;
    let params_data = RegisterOAppParams{
        delegate: *admin_acc.key
    }.try_to_vec()?;

    let mut register_data = Vec::with_capacity(8 + params_data.len());
    register_data.extend_from_slice(&register_oapp_discriminator);
    register_data.extend_from_slice(&params_data);

    // Prepare CPI account meta for the Endpoint::register_oapp instruction.
    let mut cpi_accounts: Vec<AccountMeta> = Vec::new();
    cpi_accounts.push(AccountMeta::new(*admin_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*store_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*registration_account_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        solana_program::system_program::ID,
        false,
    ));
    cpi_accounts.push(AccountMeta::new(*event_authority_acc.key, false));
    cpi_accounts.push(AccountMeta::new_readonly(*endpoint_program_acc.key, false));

    // Construct the CPI instruction to call the Endpoint program.
    let register_ix = Instruction {
        program_id: endpoint_program_id,
        accounts: cpi_accounts,
        data: register_data,
    };

    invoke_signed(
        &register_ix,
        &[
            admin_acc.clone(),
            store_acc.clone(),
            registration_account_acc.clone(),
            system_program_acc.clone(),
            event_authority_acc.clone(),
            endpoint_program_acc.clone(),
        ],
        &[&[STORE_SEED, &[store_bump]]],
    )
    .map_err(|_| ProgramCustomError::EndpointFunctionFailed)?;

    

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    admin_acc: &AccountInfo,
    store_acc: &AccountInfo,
    system_program_acc: &AccountInfo,
    event_authority_acc: &AccountInfo,
    registration_account_acc: &AccountInfo,
    endpoint_program_acc: &AccountInfo,
) -> Result<u8, ProgramError> {
    // --- Check signer ---
    if !admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate Store PDA
    let (expected_store_pubkey, store_bump) =
        Pubkey::find_program_address(&[STORE_SEED], program_id);
    if store_acc.key != &expected_store_pubkey {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate system program
    if system_program_acc.key != &solana_program::system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Validate event authority account
    let expected_event_authoriy =
        Pubkey::find_program_address(&[EVENT_SEED], endpoint_program_acc.key).0;

    if event_authority_acc.key != &expected_event_authoriy {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Validate registration account
    let expected_registration_account = Pubkey::find_program_address(
        &[OAPP_SEED, store_acc.key.as_ref()],
        endpoint_program_acc.key,
    )
    .0;

    if registration_account_acc.key != &expected_registration_account {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    Ok(store_bump)
}
