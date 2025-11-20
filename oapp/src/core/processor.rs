use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

// bring in your instruction parameter types
use crate::{
    core::{
        instruction::OAppInstruction,
        state::{InitStoreParams, SendMsgParams, SetPeerParams},
    },
    initialize::initialize_store,
    message_handlers,
    setters::set_peer,
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
            dst_oapp,
            message,
            options,
            native_fee,
            zro_fee,
        } => message_handlers::send_message::send_message(
            program_id,
            accounts,
            SendMsgParams {
                dst_eid,
                dst_oapp,
                message,
                options,
                native_fee,
                zro_fee,
            },
        ),
    }
}
