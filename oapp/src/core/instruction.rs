use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program, sysvar,
};

use crate::{
    core::state::{
        SendMsgParams, SetConfigParams, SetSendLibraryParams, ENDPOINT_SEED, LZ_COMPOSE_TYPES_SEED,
        LZ_RECEIVE_TYPES_SEED, MESSAGE_LIB_SEED, NONCE_SEED, OAPP_SEED, SEND_LIBRARY_CONFIG_SEED,
        STORE_SEED,
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
    SetSendLibrary {
        sender: Pubkey,
        eid: u32,
        new_lib: Pubkey,
    },
    SetConfig {
        oapp: Pubkey,
        eid: u32,
        config_type: u32,
        config: Vec<u8>,
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

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SetSendLibraryPayload {
    sender: Pubkey,
    eid: u32,
    new_lib: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize)]

pub struct SetConfigPayload {
    oapp: Pubkey,
    eid: u32,
    config_type: u32,
    config: Vec<u8>,
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

pub fn set_send_library(params: SetSendLibraryParams, endpoint_id: &Pubkey) -> Vec<Instruction> {
    let payload = OAppInstruction::SetSendLibrary {
        sender: params.sender,
        eid: params.eid,
        new_lib: params.new_lib,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(Pubkey::find_program_address(&[STORE_SEED], &ID).0, true),
        AccountMeta::new(
            Pubkey::find_program_address(&[OAPP_SEED, params.sender.as_ref()], endpoint_id).0,
            true,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[
                    SEND_LIBRARY_CONFIG_SEED,
                    params.sender.as_ref(),
                    &params.eid.to_be_bytes(),
                ],
                endpoint_id,
            )
            .0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[MESSAGE_LIB_SEED, &params.new_lib.to_bytes()],
                endpoint_id,
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

pub fn set_config(
    params: SetConfigParams,
    endpoint_id: &Pubkey,
    message_lib_acc: &Pubkey,
    message_lib_program: &Pubkey,
) -> Vec<Instruction> {
    let payload = OAppInstruction::SetConfig {
        oapp: params.oapp,
        eid: params.eid,
        config_type: params.config_type,
        config: params.config,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(Pubkey::find_program_address(&[STORE_SEED], &ID).0, true),
        AccountMeta::new(
            Pubkey::find_program_address(&[OAPP_SEED, params.oapp.as_ref()], endpoint_id).0,
            false,
        ),
        AccountMeta::new(
            Pubkey::find_program_address(
                &[MESSAGE_LIB_SEED, &message_lib_acc.to_bytes()],
                endpoint_id,
            )
            .0,
            false,
        ),
        //(*message_lib_acc)
        AccountMeta::new(
            Pubkey::find_program_address(&[MESSAGE_LIB_SEED], message_lib_program).0,
            false,
        ),
        AccountMeta::new(*message_lib_program, false),
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
            3 => {
                let payload = SetSendLibraryPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetSendLibrary {
                    sender: payload.sender,
                    eid: payload.eid,
                    new_lib: payload.new_lib,
                })
            }
            4 => {
                let payload = SetConfigPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetConfig {
                    oapp: payload.oapp,
                    eid: payload.eid,
                    config_type: payload.config_type,
                    config: payload.config,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
