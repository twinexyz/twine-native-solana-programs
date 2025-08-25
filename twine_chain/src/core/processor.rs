use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    append_messages::{
        append_deposit_messages, append_withdrawal_messages,
    },
    commit_finalize::{commit_and_finalize_batch, commit_batch, finalize_batch},
    core::instruction::TwineChainInstruction,
    initialize::{
        initialize_genesis_batch, initialize_message_buffer, initialize_role_manager,
        initialize_twine_chain_storage,
    },
    role::roles_manager::add_role,
    setters::{set_token_gateway, set_v_keys},
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = TwineChainInstruction::unpack(instruction_data)?;

    match instruction {
        TwineChainInstruction::InitializeRoleManager => {
            initialize_role_manager::initialize_role_manager(program_id, accounts)
        }

        TwineChainInstruction::InitializeTwineChainStorage => {
            initialize_twine_chain_storage::initialize_chain_storage(program_id, accounts)
        }

        TwineChainInstruction::InitializeMessageBuffer => {
            initialize_message_buffer::initialize_message_buffer(program_id, accounts)
        }

        TwineChainInstruction::SetTokenGateway {
            token_gateway_program,
        } => set_token_gateway::set_token_gateway(program_id, accounts, token_gateway_program),

        TwineChainInstruction::SetVkeys {
            groth16_vk,
            execution_vkey,
            inclusion_vkey,
            withdrawal_vkey,
        } => set_v_keys::set_v_keys(
            program_id,
            accounts,
            groth16_vk,
            execution_vkey,
            inclusion_vkey,
            withdrawal_vkey,
        ),

        TwineChainInstruction::AppendDepositMessage { deposit_info } => {
            append_deposit_messages::append_deposit_message(program_id, accounts, deposit_info)
        }

        TwineChainInstruction::AppendForcedWithdrawalMessage { withdraw_info } => {
            append_withdrawal_messages::append_forced_withdrawal_message(
                program_id,
                accounts,
                withdraw_info,
            )
        }

        TwineChainInstruction::InitializeGenesisBatch { genesis_batch_hash } => {
            initialize_genesis_batch::initialize_genesis_batch(
                program_id,
                accounts,
                genesis_batch_hash,
            )
        }

        TwineChainInstruction::CommitBatch {
            batch_number,
            batch_hash,
        } => commit_batch::commit_batch(program_id, accounts, batch_number, batch_hash),

        TwineChainInstruction::FinalizeBatch {
            batch_number,
            public_values,
            execution_proof,
        } => finalize_batch::finalize_batch(program_id, accounts, batch_number,public_values, execution_proof),

         TwineChainInstruction::CommitAndFinalizeBatch {
            batch_number,
            public_values,
            execution_proof,
        } => commit_and_finalize_batch::commit_and_finalize_batch(program_id, accounts, batch_number,public_values, execution_proof),

        TwineChainInstruction::AddRoleInTwineChain { address, role } => {
            add_role(program_id, accounts, address, role)
        }
    }
}
