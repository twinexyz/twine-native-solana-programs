use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

// bring in your instruction parameter types
use crate::{
    core::{
        instruction::OAppInstruction,
        state::{
            InitStoreParams, SendMsgParams, SetConfigParams, SetPeerParams, SetSendLibraryParams,
        },
    },
    initialize::initialize_store,
    message_handlers,
    setters::{set_config, set_peer, set_send_library},
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = OAppInstruction::unpack(instruction_data)?;

    match instruction {
        OAppInstruction::InitStore { endpoint_id } => {
            initialize_store::initalize_store(program_id, accounts, InitStoreParams { endpoint_id })
        }
        OAppInstruction::SetPeer {
            remote_chain,
            remote_address,
        } => set_peer::set_peer(
            program_id,
            accounts,
            SetPeerParams {
                remote_chain,
                remote_address,
            },
        ),
        OAppInstruction::SendMessage {
            dst_eid,
            receiver,
            message,
            options,
            native_fee,
            lz_token_fee,
        } => message_handlers::send_message::send_message(
            program_id,
            accounts,
            SendMsgParams {
                dst_eid,
                receiver,
                message,
                options,
                native_fee,
                lz_token_fee,
            },
        ),
        OAppInstruction::SetSendLibrary {
            sender,
            eid,
            new_lib,
        } => set_send_library::set_send_library(
            program_id,
            accounts,
            SetSendLibraryParams {
                sender,
                eid,
                new_lib,
            },
        ),
        OAppInstruction::SetConfig {
            oapp,
            eid,
            config_type,
            config,
        } => set_config::set_config(
            program_id,
            accounts,
            SetConfigParams {
                oapp,
                eid,
                config_type,
                config,
            },
        ),
    }
}
