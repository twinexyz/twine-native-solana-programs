use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    core::state::{MessageInfo, RoleType},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TwineChainInstruction {
    InitializeRoleManager,
    InitializeTwineChainStorage,
    InitializeMessageBuffer,
    SetVkeys {
        groth16_vk: Vec<u8>,
        finalize_vkey: String,
        refund_vkey: String,
        forced_withdrawal_vkey: String,
        l2_withdrawal_vkey: String,
    },
    AppendDepositMessage {
        deposit_info: MessageInfo,
    },
    AppendForcedWithdrawalMessage {
        withdraw_info: MessageInfo,
    },
    InitializeGenesisBatch {
        genesis_batch_hash: [u8; 32],
    },
    AddRoleInTwineChain {
        address: Pubkey,
        role: RoleType,
    },
    CopyMessagesBuffer,
    CommitAndFinalizeBatch {
        batch_number: u64,
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    RemoveRoleInTwineChain {
        address: Pubkey,
        role: RoleType,
    },
}

#[derive(BorshDeserialize)]
struct SetVkeysPayload {
    groth16_vk: Vec<u8>,
    finalize_vkey: String,
    refund_vkey: String,
    forced_withdrawal_vkey: String,
    l2_withdrawal_vkey: String,
}

#[derive(BorshDeserialize)]
struct AppendDepositMessagePayload {
    deposit_info: MessageInfo,
}

#[derive(BorshDeserialize)]
struct AppendForcedWithdrawalMessage {
    withdraw_info: MessageInfo,
}

#[derive(BorshDeserialize)]
struct InitializeGenesisBatchPayload {
    genesis_block_hash: [u8; 32],
}

#[derive(BorshDeserialize)]
struct CommitAndFinalizeBatchPayload {
    batch_number: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct AddRoleInTwineChainPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct RemoveRoleInTwineChainPayload {
    address: Pubkey,
    role: RoleType,
}


impl TwineChainInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        match discriminator {
            0 => Ok(Self::InitializeRoleManager),
            1 => Ok(Self::InitializeTwineChainStorage),
            2 => Ok(Self::InitializeMessageBuffer),
            3 => {
                let payload = SetVkeysPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetVkeys {
                    groth16_vk: payload.groth16_vk,
                    finalize_vkey: payload.finalize_vkey,
                    refund_vkey: payload.refund_vkey,
                    forced_withdrawal_vkey: payload.forced_withdrawal_vkey,
                    l2_withdrawal_vkey: payload.l2_withdrawal_vkey,
                })
            }
            4 => {
                let payload = AppendDepositMessagePayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AppendDepositMessage {
                    deposit_info: payload.deposit_info,
                })
            }
            5 => {
                let payload = AppendForcedWithdrawalMessage::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AppendForcedWithdrawalMessage {
                    withdraw_info: payload.withdraw_info,
                })
            }
            6 => {
                let payload = InitializeGenesisBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::InitializeGenesisBatch {
                    genesis_batch_hash: payload.genesis_block_hash,
                })
            }
            7 => {
                let payload = AddRoleInTwineChainPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AddRoleInTwineChain {
                    address: payload.address,
                    role: payload.role,
                })
            }
            8 => Ok(Self::CopyMessagesBuffer),
            9 => {
                let payload = CommitAndFinalizeBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::CommitAndFinalizeBatch {
                    batch_number: payload.batch_number,
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            10 => {
                msg!("Here in remove role");
                let payload = RemoveRoleInTwineChainPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveRoleInTwineChain {
                    address: payload.address,
                    role: payload.role,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
