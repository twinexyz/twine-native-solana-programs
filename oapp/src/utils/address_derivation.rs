use solana_program::pubkey::Pubkey;

use crate::utils::constants::*;

pub fn derive_store_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STORE_SEED], program_id)
}

pub fn derive_event_authority(endpoint_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_SEED], endpoint_program_id)
}

pub fn derive_oapp_registry(endpoint_program_id: &Pubkey, store_pda: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[OAPP_SEED, &store_pda.as_ref()], endpoint_program_id)
}
