use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::core::{
    error::ProgramCustomError,
    state::{PeerConfig, SetPeerParams, Store, PEER_SEED, STORE_SEED},
};

pub fn set_peer(program_id: &Pubkey, accounts: &[AccountInfo], params: SetPeerParams) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let admin_acc = next_account_info(account_info_iter)?;
    let peer_acc = next_account_info(account_info_iter)?;
    let store_acc = next_account_info(account_info_iter)?;

    if !admin_acc.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_store, _bump) = Pubkey::find_program_address(&[STORE_SEED], program_id);
    if store_acc.key != &expected_store {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    let store_data = Store::try_from_slice(&store_acc.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    if *admin_acc.key != store_data.admin {
        return Err(ProgramError::UninitializedAccount);
    }

    // Derive expected peer PDA
    let chain_id_bytes = &params.remote_chain.to_be_bytes();
    let seed_parts = &[PEER_SEED, store_acc.key.as_ref(), chain_id_bytes];
    let (expected_peer, peer_bump) = Pubkey::find_program_address(seed_parts, program_id);

    if peer_acc.key != &expected_peer || peer_acc.owner != program_id {
        return Err(ProgramCustomError::InvalidPDA.into());
    }

    // Initialize or update the Peer account data
    let mut peer_data =
        PeerConfig::try_from_slice(&peer_acc.data.borrow()).unwrap_or_else(|_| PeerConfig {
            peer_address: [0u8; 32],
            bump: 0,
        });
    peer_data.peer_address = params.remote_address;
    peer_data.bump = peer_bump;
    peer_data.serialize(&mut *peer_acc.data.borrow_mut())?;

    Ok(())
}
