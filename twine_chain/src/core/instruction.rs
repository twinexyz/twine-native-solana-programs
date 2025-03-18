use crate::core::state::{CommitBatchInfo, DepositMessageInfo, ForcedWithdrawMessageInfo};
use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub enum TwineChainInstruction {
    InitializeRoleManager,
    InitializeTwineChainStorage,
    InitializeMessageBuffer,
    SetTokenGateway {
        token_gateway_program: Pubkey,
    },
    SetVkeys {
        groth16_vk: Vec<u8>,
        execution_vkey: String,
        inclusion_vkey: String,
        withdrawal_vkey: String,
    },
    AppendDepositMessage {
        deposit_info: DepositMessageInfo,
    },
    AppendForcedWithdrawalMessage {
        withdraw_info: ForcedWithdrawMessageInfo,
    },
    InitializeGenesisBatch {
        genesis_block_hash: [u8; 32],
    },
    RemoveWithdrawalMessage {
        nonce: u64,
    },
    CommitBatch {
        start_block: u64,
        end_block: u64,
        batch_data: Vec<CommitBatchInfo>,
    },
    FinalizeBatch {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    CommitAndFinalizeTransaction {
        transaction_info: Vec<u8>,
        inclusion_proof: Vec<u8>,
    },
}

#[derive(BorshDeserialize)]
struct SetTokenGatewayPayload {
    token_gateway_program: Pubkey,
}

#[derive(BorshDeserialize)]
struct SetVkeysPayload {
    groth16_vk: Vec<u8>,
    execution_vkey: String,
    inclusion_vkey: String,
    withdrawal_vkey: String,
}

#[derive(BorshDeserialize)]
struct AppendDepositMessagePayload {
    deposit_info: DepositMessageInfo,
}

#[derive(BorshDeserialize)]
struct AppendForcedWithdrawalMessage {
    withdraw_info: ForcedWithdrawMessageInfo,
}

#[derive(BorshDeserialize)]
struct InitializeGenesisBatchPayload {
    genesis_block_hash: [u8; 32],
}

#[derive(BorshDeserialize)]
struct RemoveWithdrawalMessagePayload {
    nonce: u64,
}

#[derive(BorshDeserialize)]
struct CommitBatchPayload {
    start_block: u64,
    end_block: u64,
    batch_data: Vec<CommitBatchInfo>,
}

#[derive(BorshDeserialize)]
struct FinalizeBatchPayload {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshDeserialize)]
struct CommitAndFinalizeTransactionPayload {
    transaction_info: Vec<u8>,
    inclusion_proof: Vec<u8>,
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
                let payload = SetTokenGatewayPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetTokenGateway {
                    token_gateway_program: payload.token_gateway_program,
                })
            }
            4 => {
                let payload = SetVkeysPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetVkeys {
                    groth16_vk: payload.groth16_vk,
                    execution_vkey: payload.execution_vkey,
                    inclusion_vkey: payload.inclusion_vkey,
                    withdrawal_vkey: payload.withdrawal_vkey,
                })
            }
            5 => {
                let payload = AppendDepositMessagePayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AppendDepositMessage {
                    deposit_info: payload.deposit_info,
                })
            }
            6 => {
                let payload = AppendForcedWithdrawalMessage::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AppendForcedWithdrawalMessage {
                    withdraw_info: payload.withdraw_info,
                })
            }
            7 => {
                let payload = InitializeGenesisBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::InitializeGenesisBatch {
                    genesis_block_hash: payload.genesis_block_hash,
                })
            }
            8 => {
                let payload = RemoveWithdrawalMessagePayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveWithdrawalMessage {
                    nonce: payload.nonce,
                })
            }
            9 => {
                let payload = CommitBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::CommitBatch {
                    start_block: payload.start_block,
                    end_block: payload.end_block,
                    batch_data: payload.batch_data,
                })
            }
            10 => {
                let payload = FinalizeBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::FinalizeBatch {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            11 => {
                let payload = CommitAndFinalizeTransactionPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::CommitAndFinalizeTransaction {
                    transaction_info: payload.transaction_info,
                    inclusion_proof: payload.inclusion_proof,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
