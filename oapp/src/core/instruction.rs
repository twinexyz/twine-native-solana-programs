use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program, sysvar,
};

use crate::{
    core::state::{
        SendMsgParams, ENDPOINT_SEED, LZ_COMPOSE_TYPES_SEED, LZ_RECEIVE_TYPES_SEED,
        MESSAGE_LIB_SEED, NONCE_SEED, SEND_LIBRARY_CONFIG_SEED, STORE_SEED,
    },
    ID,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum OAppInstruction {
    InitStore {
        endpoint_id: Pubkey,
    },
    SetPeer {
        remote_chain: u32,
        remote_address: [u8; 32],
    },
    SendMessage {
        dst_eid: u32,
        dst_oapp: [u8; 32],
        message: Vec<u8>,
        options: Vec<u8>,
        native_fee: u64,
        zro_fee: u64,
    },
}

#[derive(BorshSerialize, BorshDeserialize)]
struct InitStorePayload {
    endpoint_id: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct SetPeerPayload {
    remote_chain: u32,
    remote_address: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize)]
struct SendMessagePayload {
    dst_eid: u32,
    dst_oapp: [u8; 32],
    message: Vec<u8>,
    options: Vec<u8>,
    native_fee: u64,
    zro_fee: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct LzReceivePayload {
    src_chain: u32,
    sender: [u8; 32],
    nonce: u64,
    guid: [u8; 32],
    message: Vec<u8>,
}

pub fn initialize_store(endpoint_id: &Pubkey, admin: &Pubkey) -> Vec<Instruction> {
    let payload = OAppInstruction::InitStore {
        endpoint_id: *endpoint_id,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = Pubkey::find_program_address(&[STORE_SEED], &ID).0;

    let accounts = vec![
        AccountMeta::new(*admin, true),
        AccountMeta::new(Pubkey::find_program_address(&[STORE_SEED], &ID).0, true),
        AccountMeta::new(
            Pubkey::find_program_address(&[LZ_RECEIVE_TYPES_SEED, store_account.as_ref()], &ID).0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(&[LZ_COMPOSE_TYPES_SEED, store_account.as_ref()], &ID).0,
            false,
        ),
        AccountMeta::new(system_program::id(), false),
        AccountMeta::new(sysvar::rent::ID, false),
        AccountMeta::new(
            Pubkey::find_program_address(&[b"OApp", store_account.as_ref()], &ID).0,
            false,
        ),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn send_message(
    params: SendMsgParams,
    endpoint_id: &Pubkey,
    send_library_id: &Pubkey,
) -> Vec<Instruction> {
    let payload = OAppInstruction::SendMessage {
        dst_eid: params.dst_eid,
        dst_oapp: params.dst_oapp,
        message: params.message,
        options: params.options,
        native_fee: params.native_fee,
        zro_fee: params.zro_fee,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = Pubkey::find_program_address(&[STORE_SEED], &ID).0;

    let accounts = vec![
        AccountMeta::new(*endpoint_id, false),
        AccountMeta::new(Pubkey::find_program_address(&[STORE_SEED], &ID).0, true),
        AccountMeta::new(*send_library_id, false),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[
                    SEND_LIBRARY_CONFIG_SEED,
                    store_account.as_ref(),
                    &params.dst_eid.to_be_bytes(),
                ],
                &endpoint_id,
            )
            .0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[SEND_LIBRARY_CONFIG_SEED, &params.dst_eid.to_be_bytes()],
                &endpoint_id,
            )
            .0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[MESSAGE_LIB_SEED, &send_library_id.to_bytes()],
                &endpoint_id,
            )
            .0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(&[ENDPOINT_SEED], &endpoint_id).0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[
                    NONCE_SEED,
                    &store_account.to_bytes(),
                    &params.dst_eid.to_be_bytes(),
                    &params.dst_oapp[..],
                ],
                &endpoint_id,
            )
            .0,
            false,
        ),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

impl OAppInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match discriminator {
            0 => {
                let payload = InitStorePayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::InitStore {
                    endpoint_id: payload.endpoint_id,
                })
            }
            1 => {
                let payload = SetPeerPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetPeer {
                    remote_chain: payload.remote_chain,
                    remote_address: payload.remote_address,
                })
            }
            2 => {
                let payload = SendMessagePayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SendMessage {
                    dst_eid: payload.dst_eid,
                    dst_oapp: payload.dst_oapp,
                    message: payload.message,
                    options: payload.options,
                    native_fee: payload.native_fee,
                    zro_fee: payload.zro_fee,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
