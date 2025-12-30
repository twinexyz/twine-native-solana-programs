use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use crate::{
    core::{
        error::ProgramCustomError,
        state::{LayerZeroInfo, TwineChainRoleManager},
    },
    utils::{
        address_derivation::{
            derive_layer_zero_info, derive_twine_chain_role_manager, verify_derived_address,
            verify_system_program,
        },
        constants::LAYER_ZERO_INFO_PREFIX,
    },
};

pub fn initialize_layer_zero_info(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let layer_zero_info_acc = next_account_info(account_iter)?;
    let role_manager_acc = next_account_info(account_iter)?;
    let chain_admin_acc = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    // Derive and validate PDA
    let layer_zero_info_bump = validate_accounts(
        program_id,
        layer_zero_info_acc,
        role_manager_acc,
        chain_admin_acc,
        system_program,
    )?;

    let rent = Rent::default();

    // Create account
    if layer_zero_info_acc.data_is_empty() {
        let layer_zero_info_space = LayerZeroInfo::LEN;
        let required_lamports = rent.minimum_balance(layer_zero_info_space);
        let create_ix = system_instruction::create_account(
            chain_admin_acc.key,
            layer_zero_info_acc.key,
            required_lamports,
            layer_zero_info_space as u64,
            program_id,
        );
        invoke_signed(
            &create_ix,
            &[
                chain_admin_acc.clone(),
                layer_zero_info_acc.clone(),
                system_program.clone(),
            ],
            &[&[LAYER_ZERO_INFO_PREFIX.as_bytes(), &[layer_zero_info_bump]]],
        )?;
    }

    let receiver_address = [0u8; 32];
    let options_hex = "0x00030100110100000000000000000000000000000000";
    let options = hex::decode(options_hex.trim_start_matches("0x")).unwrap();

    let layer_zero_info_data = LayerZeroInfo {
        is_initialized: true,
        dst_eid: 40161,
        receiver: receiver_address,
        options: options,
        native_fee: 0,
        lz_token_fee: 0,
    };

    layer_zero_info_data
        .serialize(&mut &mut layer_zero_info_acc.data.borrow_mut()[..])
        .map_err(|_| ProgramCustomError::SerializeFailed)?;

    msg!("Layer Zero Info Initialized");
    Ok(())
}

fn validate_accounts(
    program_id: &Pubkey,
    layer_zero_info_acc: &AccountInfo,
    role_manager_acc: &AccountInfo,
    chain_admin_acc: &AccountInfo,
    system_program: &AccountInfo,
) -> Result<u8, ProgramError> {
    // Validate signer
    if !chain_admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_layer_zero_info, layer_zero_info_bump) = derive_layer_zero_info(program_id);
    verify_derived_address(expected_layer_zero_info, layer_zero_info_acc)?;

    let (expected_role_manager_pda, _) = derive_twine_chain_role_manager(program_id);
    verify_derived_address(expected_role_manager_pda, role_manager_acc)?;

    verify_system_program(system_program)?;

    if !layer_zero_info_acc.data_is_empty() {
        let layer_zero_info_data: LayerZeroInfo =
            LayerZeroInfo::deserialize(&mut &layer_zero_info_acc.data.borrow()[..])
                .map_err(|_| ProgramError::InvalidAccountData)?;

        if layer_zero_info_data.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // Checks if signer has required role(ChainAdmin)
    let role_manager_data =
        TwineChainRoleManager::deserialize(&mut &role_manager_acc.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if role_manager_data.chain_admin != *chain_admin_acc.key {
        return Err(ProgramCustomError::Unauthorized.into());
    }

    Ok(layer_zero_info_bump)
}
