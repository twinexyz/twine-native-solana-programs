use crate::{
    core::state::{
        InitConfigParams, InitNonceParams, InitReceiveLibraryParams, InitSendLibraryParams,
        SendMsgParams, SetConfigParams, SetSendLibraryParams, DVN_CONFIG_SEED,
        ENDPOINT_INIT_CONFIG_DISCRIMINATOR, ENDPOINT_INIT_NONCE_DISCRIMINATOR,
        ENDPOINT_INIT_RECEIVE_LIBRARY_DISCRIMINATOR, ENDPOINT_INIT_SEND_LIBRARY_DISCRIMINATOR,
        EXECUTOR_CONFIG_SEED,
    },
    utils::address_derivation::*,
    ID,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};
use std::{str::FromStr, vec};

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
        receiver: [u8; 32],
        message: Vec<u8>,
        options: Vec<u8>,
        native_fee: u64,
        lz_token_fee: u64,
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
    receiver: [u8; 32],
    message: Vec<u8>,
    options: Vec<u8>,
    native_fee: u64,
    lz_token_fee: u64,
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

pub fn initialize_store(admin: &Pubkey) -> Vec<Instruction> {
    let payload = OAppInstruction::InitStore {
        endpoint_id: get_endpoint_id(),
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = derive_store_pda(&ID).0;

    let accounts = vec![
        AccountMeta::new(*admin, true),
        AccountMeta::new(store_account, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        AccountMeta::new(derive_oapp_registry(&store_account).0, false),
        AccountMeta::new_readonly(get_endpoint_id(), false),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn send_message(
    params: SendMsgParams,
    admin: &Pubkey,
    dvn_program: &Pubkey,
    executor_program: &Pubkey,
) -> Vec<Instruction> {
    let payload = OAppInstruction::SendMessage {
        dst_eid: params.dst_eid,
        receiver: params.receiver,
        message: params.message,
        options: params.options,
        native_fee: params.native_fee,
        lz_token_fee: params.lz_token_fee,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = derive_store_pda(&ID).0;

    let native_loader_program_id =
        Pubkey::from_str("NativeLoader1111111111111111111111111111111").unwrap();

    let accounts = vec![
        // <------------------- Endpoint Accounts --------------------------->
        // sender
        AccountMeta::new(store_account, false),
        // sendLibraryProgram (ULN Program)
        AccountMeta::new_readonly(get_send_library_program(), false),
        // sendLibraryConfig
        AccountMeta::new(
            derive_send_library_config(&store_account, &params.dst_eid).0,
            false,
        ),
        // defaultSendLibraryConfig
        AccountMeta::new(derive_default_send_library_config(&params.dst_eid).0, false),
        // sendLibraryInfo (sendLibrary: 2Xg...LkQ)
        AccountMeta::new_readonly(derive_send_library_info().0, false),
        // endpointSettings
        AccountMeta::new(derive_endpoint_settings().0, false),
        // nonce
        AccountMeta::new(
            derive_nonce(&store_account, &params.dst_eid, &params.receiver).0,
            false,
        ),
        // eventAuthority
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_endpoint_id(), false),
        // <------------------- Library Accounts --------------------------->
        // uln
        AccountMeta::new(derive_uln().0, false),
        // sendConfig
        AccountMeta::new(derive_send_config(&params.dst_eid, &store_account).0, false),
        // defaultSendConfig
        AccountMeta::new(derive_default_send_config(&params.dst_eid).0, false),
        // payer
        AccountMeta::new(*admin, true),
        // treasury (Optional)
        AccountMeta::new(*admin, false),
        // systemProgram
        AccountMeta::new_readonly(system_program::ID, false),
        // eventAuthority
        AccountMeta::new(derive_library_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_send_library_program(), false),
        // <------------------ Remaining Accounts ------------------------->
        // Executor Program
        AccountMeta::new_readonly(*executor_program, false),
        // Executor Config
        AccountMeta::new(
            Pubkey::find_program_address(&[EXECUTOR_CONFIG_SEED], executor_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
        // DVN program
        AccountMeta::new_readonly(*dvn_program, false),
        // dvn config
        AccountMeta::new(
            Pubkey::find_program_address(&[DVN_CONFIG_SEED], dvn_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn set_send_library(params: SetSendLibraryParams) -> Vec<Instruction> {
    let payload = OAppInstruction::SetSendLibrary {
        sender: params.sender,
        eid: params.eid,
        new_lib: params.new_lib,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(derive_store_pda(&ID).0, false),
        AccountMeta::new(derive_oapp_registry(&params.sender).0, false),
        AccountMeta::new(
            derive_send_library_config(&params.sender, &params.eid).0,
            false,
        ),
        AccountMeta::new(derive_message_lib_info(&params.new_lib).0, false),
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        AccountMeta::new_readonly(get_endpoint_id(), false),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn set_config(params: SetConfigParams) -> Vec<Instruction> {
    let store_account = derive_store_pda(&ID).0;
    let message_lib = derive_message_lib().0;

    let payload = OAppInstruction::SetConfig {
        oapp: params.oapp,
        eid: params.eid,
        config_type: params.config_type,
        config: params.config,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(store_account, false),
        AccountMeta::new(derive_oapp_registry(&store_account).0, false),
        // messageLibInfo
        AccountMeta::new_readonly(derive_message_lib_info(&message_lib).0, false),
        // message lib
        AccountMeta::new(message_lib, false),
        // messageLibProgram
        AccountMeta::new(get_send_library_program(), false),
        // --------------- remaining accounts for ULN::init_config ---------------
        // uln
        AccountMeta::new_readonly(derive_uln().0, false),
        // send_config
        AccountMeta::new(derive_send_config(&params.eid, &params.oapp).0, false),
        // receive_config
        AccountMeta::new(derive_receive_config(&params.eid, &params.oapp).0, false),
        // default_send_config
        AccountMeta::new(derive_default_send_config(&params.eid).0, false),
        // default receive_config
        AccountMeta::new(derive_default_receive_config(&params.eid).0, false),
        // event account
        AccountMeta::new(derive_library_event_authority().0, false),
        // program
        AccountMeta::new(get_send_library_program(), false),
        AccountMeta::new(get_endpoint_id(), false),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

// Instruction to call the enpoint's function directly:
pub fn init_send_library(admin: &Pubkey) -> Vec<Instruction> {
    let store_account = derive_store_pda(&ID).0;

    let payload = InitSendLibraryParams {
        sender: store_account,
        eid: 40161,
    };

    let discriminator = ENDPOINT_INIT_SEND_LIBRARY_DISCRIMINATOR;
    let params_data = payload.try_to_vec().unwrap();

    let mut data = vec![];
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&params_data);

    let accounts = vec![
        AccountMeta::new(*admin, true),
        AccountMeta::new(derive_oapp_registry(&store_account).0, false),
        AccountMeta::new(
            derive_send_library_config(&payload.sender, &payload.eid).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: get_endpoint_id(),
        accounts,
        data,
    }]
}

pub fn init_receive_library(admin: &Pubkey) -> Vec<Instruction> {
    let store_account = derive_store_pda(&ID).0;

    let payload = InitReceiveLibraryParams {
        receiver: store_account,
        eid: 40161,
    };

    let discriminator = ENDPOINT_INIT_RECEIVE_LIBRARY_DISCRIMINATOR;
    let params_data = payload.try_to_vec().unwrap();

    let mut data = vec![];
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&params_data);

    let accounts = vec![
        AccountMeta::new(*admin, true),
        AccountMeta::new(derive_oapp_registry(&store_account).0, false),
        AccountMeta::new(
            derive_receive_library_config(&payload.receiver, &payload.eid).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: get_endpoint_id(),
        accounts,
        data,
    }]
}

pub fn init_nonce(admin: &Pubkey, remote_oapp: [u8; 32]) -> Vec<Instruction> {
    let store_account = derive_store_pda(&ID).0;

    let payload = InitNonceParams {
        local_oapp: store_account,
        remote_eid: 40161,
        remote_oapp: remote_oapp,
    };

    let discriminator = ENDPOINT_INIT_NONCE_DISCRIMINATOR;
    let params_data = payload.try_to_vec().unwrap();

    let mut data = vec![];
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&params_data);

    let accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(*admin, true),
        AccountMeta::new(derive_oapp_registry(&store_account).0, false),
        AccountMeta::new(
            derive_nonce(
                &payload.local_oapp,
                &payload.remote_eid,
                &payload.remote_oapp,
            )
            .0,
            false,
        ),
        AccountMeta::new(
            derive_pending_inbound_nonce(
                &payload.local_oapp,
                &payload.remote_eid,
                &payload.remote_oapp,
            )
            .0,
            false,
        ),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: get_endpoint_id(),
        accounts,
        data,
    }]
}

pub fn init_config(admin: &Pubkey) -> Vec<Instruction> {
    let store_account = derive_store_pda(&ID).0;
    let message_lib = derive_message_lib().0;

    let payload = InitConfigParams {
        oapp: store_account,
        eid: 40161,
    };

    let discriminator = ENDPOINT_INIT_CONFIG_DISCRIMINATOR;
    let params_data = payload.try_to_vec().unwrap();

    let mut data = vec![];
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&params_data);

    let accounts = vec![
        AccountMeta::new(*admin, true),
        AccountMeta::new(derive_oapp_registry(&store_account).0, false),
        // messageLibInfo
        AccountMeta::new_readonly(derive_message_lib_info(&message_lib).0, false),
        // message lib
        AccountMeta::new(message_lib, false),
        // messageLibProgram
        AccountMeta::new(get_send_library_program(), false),
        // --------------- remaining accounts for ULN::init_config ---------------
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(derive_message_lib().0, false),
        AccountMeta::new(derive_send_config(&payload.eid, &payload.oapp).0, false),
        AccountMeta::new(derive_receive_config(&payload.eid, &payload.oapp).0, false),
        AccountMeta::new(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: get_endpoint_id(),
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
                    receiver: payload.receiver,
                    message: payload.message,
                    options: payload.options,
                    native_fee: payload.native_fee,
                    lz_token_fee: payload.lz_token_fee,
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
