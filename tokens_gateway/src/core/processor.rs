use solana_program::{account_info::AccountInfo,msg, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::core::error::ProgramCustomError;
use crate::core::instruction::GatewayInstruction;
use crate::finalize_withdrawal::{finalize_native_withdrawal, finalize_spl_withdrawal};
use crate::initialize::{initialize_tokens_gateway, initialze_role_manager};
use crate::native::{native_deposit, native_forced_withdrawal};
use crate::roles::roles_manager::{add_role, remove_role, set_role_chain_admin};
use crate::setters::update_token_mapping;
use crate::spl::{spl_deposit, spl_forced_withdrawal};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("process_instruction");
    // Parse the instruction.
    let instruction = GatewayInstruction::unpack_instruction(instruction_data)
        .map_err(|_| ProgramCustomError::InvalidInstructionData)?;

    match instruction {
        GatewayInstruction::InitializeTokensGatewayRoleManager => {
            initialze_role_manager::initialize_role_manager(program_id, accounts)
        }
        GatewayInstruction::InitializeTokensGateway => {
            initialize_tokens_gateway::initialize_tokens_gateway(program_id, accounts)
        }

        GatewayInstruction::UpdateTokenMapping {
            l1_token,
            l2_token,
            l1_decimals,
            l2_decimals,
        } => update_token_mapping::update_token_mapping(
            program_id,
            accounts,
            l1_token,
            l2_token,
            l1_decimals,
            l2_decimals,
        ),
        GatewayInstruction::SetGatewayRoleChainAdmin { new_admin } => {
            set_role_chain_admin(program_id, accounts, new_admin)
        }
        GatewayInstruction::AddRoleInGateway { address, role } => {
            add_role(program_id, accounts, address, role)
        }
        GatewayInstruction::RemoveRoleInGateway { address, role } => {
            remove_role(program_id, accounts, address, role)
        }
        GatewayInstruction::NativeTokenDepoist {
            receiver_twine_address,
            l1_token,
            l2_token,
            amount,
        } => native_deposit::native_token_deposit(
            program_id,
            accounts,
            receiver_twine_address,
            l1_token,
            l2_token,
            amount,
        ),
        GatewayInstruction::SplTokenDepoist {
            receiver_twine_address,
            l1_token,
            l2_token,
            amount,
        } => spl_deposit::spl_token_deposit(
            program_id,
            accounts,
            receiver_twine_address,
            l1_token,
            l2_token,
            amount,
        ),
        GatewayInstruction::NativeTokenForcedWithdrawal {
            from_twine_address,
            to_l1_pubkey,
            l1_token,
            l2_token,
            amount,
            signature,
        } => native_forced_withdrawal::forced_native_token_withdrawal(
            program_id,
            accounts,
            from_twine_address,
            to_l1_pubkey,
            l1_token,
            l2_token,
            amount,
            signature,
        ),
        GatewayInstruction::SplTokenForcedWithdrawal {
            from_twine_address,
            to_l1_pubkey,
            l1_token,
            l2_token,
            amount,
            signature,
        } => spl_forced_withdrawal::forced_spl_token_withdrawal(
            program_id,
            accounts,
            from_twine_address,
            to_l1_pubkey,
            l1_token,
            l2_token,
            amount,
            signature,
        ),
        GatewayInstruction::FinalzeNativeWithdrawal { withdrawal_inputs } => {
            finalize_native_withdrawal::finalize_native_withdrawal(
                program_id,
                accounts,
                withdrawal_inputs,
            )
        }
        GatewayInstruction::FinalizeSplWithdrawal { withdrawal_inputs } => {
            finalize_spl_withdrawal::finalize_spl_withdrawal(
                program_id,
                accounts,
                withdrawal_inputs,
            )
        }
    }
}
