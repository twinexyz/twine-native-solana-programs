use crate::core::{
    error::ProgramCustomError,
    state::{
        ENDPOINT_REGISTER_OAPP_DISCRIMINATOR, InitStoreParams, LZ_COMPOSE_TYPES_SEED, LZ_RECEIVE_TYPES_SEED, STORE_SEED, Store, TypesAccount
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

pub fn initalize_store(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: InitStoreParams,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let admin_acc = next_account_info(account_info_iter)?;
    let store_acc = next_account_info(account_info_iter)?;
    let lz_receive_type_acc = next_account_info(account_info_iter)?;
    let lz_compose_type_acc = next_account_info(account_info_iter)?;

    // Accounts used to call Endpoint::register_oapp
    let system_program_acc = next_account_info(account_info_iter)?;
    let rent_sysvar_acc = next_account_info(account_info_iter)?;
    let registration_account_acc = next_account_info(account_info_iter)?;

    let store_bump = validate_accounts(
        program_id,
        admin_acc,
        store_acc,
        lz_receive_type_acc,
        lz_compose_type_acc,
        system_program_acc,
        rent_sysvar_acc,
    )?;

    // At this point, the PDAs should be new (zeroed) and rent-exempt (client must fund them).
    // Initialize their data:
    let mut store_data =
        Store::try_from_slice(&store_acc.data.borrow()).unwrap_or_else(|_| Store {
            // if not yet initialized, create default
            admin: Pubkey::default(),
            endpoint_program: Pubkey::default(),
            bump: 0,
            last_msg_size: 0,
            last_msg: [0u8; 256],
        });

    // Set store fields
    store_data.admin = *admin_acc.key;
    store_data.endpoint_program = params.endpoint_id;
    store_data.bump = store_bump;
    store_data.last_msg_size = 0;
    store_data.last_msg = [0u8; 256];

    store_data.serialize(&mut *store_acc.data.borrow_mut())?;

    // Initialize the types accounts:
    let mut recv_types_data = TypesAccount::try_from_slice(&lz_receive_type_acc.data.borrow())
        .unwrap_or_else(|_| TypesAccount {
            store: Pubkey::default(),
        });
    recv_types_data.store = *store_acc.key;
    recv_types_data.serialize(&mut *lz_receive_type_acc.data.borrow_mut())?;

    let mut comp_types_data = TypesAccount::try_from_slice(&lz_compose_type_acc.data.borrow())
        .unwrap_or_else(|_| TypesAccount {
            store: Pubkey::default(),
        });
    comp_types_data.store = *store_acc.key;
    comp_types_data.serialize(&mut *lz_compose_type_acc.data.borrow_mut())?;

    // ----------------- Register the OApp with LayerZero Endpoint via CPI -----------------

    let endpoint_program_id = params.endpoint_id;
    let register_delegate = admin_acc.key;

    // Prepare the instruction data for Endpoint::register_oapp.
    // Anchor instruction use an 8-byte discriminator followed by the serialized parameters.
    // Discriminator for "register_oapp" (sha256("global:register_oapp")[..8]):

    let register_oapp_discriminator =  ENDPOINT_REGISTER_OAPP_DISCRIMINATOR;

    let mut register_data = Vec::with_capacity(8 + 32);
    register_data.extend_from_slice(&register_oapp_discriminator);
    register_data.extend_from_slice(register_delegate.as_ref());

    // Determine the PDA for the Endpoint program's OApp registration account.
    let reg_seed = &[b"OApp", store_acc.key.as_ref()];
    let (registration_pda, _reg_bump) =
        Pubkey::find_program_address(reg_seed, &endpoint_program_id);

    if registration_account_acc.key != &registration_pda {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Prepare CPI account meta for the Endpoint::register_oapp instruction.
    let mut cpi_accounts: Vec<AccountMeta> = Vec::new();
    cpi_accounts.push(AccountMeta::new(*admin_acc.key, true));
    cpi_accounts.push(AccountMeta::new(*store_acc.key, true));
    cpi_accounts.push(AccountMeta::new(registration_pda, false));
    cpi_accounts.push(AccountMeta::new_readonly(
        solana_program::system_program::ID,
        false,
    ));
    cpi_accounts.push(AccountMeta::new_readonly(sysvar::rent::ID, false));

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
            rent_sysvar_acc.clone(),
        ],
        &[&[STORE_SEED, &[store_bump]]],
    )?;

    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    admin_acc: &AccountInfo,
    store_acc: &AccountInfo,
    lz_receive_type_acc: &AccountInfo,
    lz_compose_type_acc: &AccountInfo,
    system_program_acc: &AccountInfo,
    rent_sysvar_acc: &AccountInfo,
) -> Result<u8, ProgramError> {
    // --- Check signer ---
    if !admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_store_pubkey, store_bump) =
        Pubkey::find_program_address(&[STORE_SEED], program_id);
    if store_acc.key != &expected_store_pubkey || store_acc.owner != program_id {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let seed_for_types = &[LZ_RECEIVE_TYPES_SEED, store_acc.key.as_ref()];
    let (expected_lz_recv_pubkey, _types_bump) =
        Pubkey::find_program_address(seed_for_types, program_id);
    if lz_receive_type_acc.key != &expected_lz_recv_pubkey
        || lz_receive_type_acc.owner != program_id
    {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let seed_for_comp = &[LZ_COMPOSE_TYPES_SEED, store_acc.key.as_ref()];
    let (expected_lz_comp_pubkey, _comp_bump) =
        Pubkey::find_program_address(seed_for_comp, program_id);
    if lz_compose_type_acc.key != &expected_lz_comp_pubkey
        || lz_compose_type_acc.owner != program_id
    {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    if system_program_acc.key != &solana_program::system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    if rent_sysvar_acc.key != &sysvar::rent::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(store_bump)
}
