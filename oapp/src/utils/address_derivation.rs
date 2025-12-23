use std::str::FromStr;

use solana_program::pubkey::Pubkey;

use crate::utils::constants::*;

pub fn derive_store_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STORE_SEED], program_id)
}

// <------------------------------ Endpoint Accounts ------------------------------>
pub fn derive_endpoint_event_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_SEED], &get_endpoint_id())
}

pub fn derive_oapp_registry(store_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[OAPP_SEED, store_pda.as_ref()], &get_endpoint_id())
}

pub fn derive_send_library_config(store_pda: &Pubkey, dst_eid: &u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEND_LIBRARY_CONFIG_SEED,
            store_pda.as_ref(),
            &dst_eid.to_be_bytes(),
        ],
        &get_endpoint_id(),
    )
}

pub fn derive_default_send_library_config(dst_eid: &u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEND_LIBRARY_CONFIG_SEED, &dst_eid.to_be_bytes()],
        &get_endpoint_id(),
    )
}

pub fn derive_receive_library_config(store_pda: &Pubkey, dst_eid: &u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RECEIVE_LIBRARY_CONFIG_SEED,
            store_pda.as_ref(),
            &dst_eid.to_be_bytes(),
        ],
        &get_endpoint_id(),
    )
}


pub fn derive_send_library_info() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MESSAGE_LIB_SEED, &get_send_library().to_bytes()],
        &get_endpoint_id(),
    )
}

pub fn derive_endpoint_settings() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ENDPOINT_SEED], &get_endpoint_id())
}

pub fn derive_nonce(store_pda: &Pubkey, dst_eid: &u32, receiver: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            NONCE_SEED,
            &store_pda.to_bytes(),
            &dst_eid.to_be_bytes(),
            &receiver[..],
        ],
        &get_endpoint_id(),
    )
}

pub fn derive_pending_inbound_nonce(store_pda: &Pubkey, dst_eid: &u32, receiver: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PENDING_NONCE_SEED,
            &store_pda.to_bytes(),
            &dst_eid.to_be_bytes(),
            &receiver[..],
        ],
        &get_endpoint_id(),
    )
}

pub fn derive_message_lib_info(message_lib: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MESSAGE_LIB_SEED, &message_lib.to_bytes()],
        &get_endpoint_id(),
    )
}

// <------------------------------ Library Accounts ------------------------------>
pub fn derive_uln() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ULN_SEED], &get_send_library_program())
}

pub fn derive_send_config(dst_eid: &u32, store_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEND_CONFIG_SEED,
            &dst_eid.to_be_bytes(),
            &store_pda.to_bytes(),
        ],
        &get_send_library_program(),
    )
}

pub fn derive_default_send_config(dst_eid: &u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEND_CONFIG_SEED, &dst_eid.to_be_bytes()],
        &get_send_library_program(),
    )
}

pub fn derive_receive_config(dst_eid: &u32, store_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            RECEIVE_CONFIG_SEED,
            &dst_eid.to_be_bytes(),
            &store_pda.to_bytes(),
        ],
        &get_send_library_program(),
    )
}

pub fn derive_default_receive_config(dst_eid: &u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[RECEIVE_CONFIG_SEED, &dst_eid.to_be_bytes()],
        &get_send_library_program(),
    )
}

pub fn derive_library_event_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_SEED], &get_send_library_program())
}

pub fn derive_message_lib() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MESSAGE_LIB_SEED], &get_send_library_program())
}

// Getters
pub fn get_endpoint_id() -> Pubkey {
    Pubkey::from_str(ENDPOINT_ID).unwrap()
}

pub fn get_send_library_program() -> Pubkey {
    Pubkey::from_str(SEND_LIBRARY_PROGRAM_ID).unwrap()
}

pub fn get_send_library() -> Pubkey {
    Pubkey::from_str(SEND_LIBRARY_INFO_ID).unwrap()
}
