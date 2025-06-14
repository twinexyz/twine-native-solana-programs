use std::vec;

use crate::core::state::{
    CommitBatchInfo, DepositMessageInfo, ForcedWithdrawMessageInfo, RoleType,
};
use crate::utils::address_derivation::{
    derive_commitment_pda, derive_deposit_message_buffer, derive_execution_message_buffer,
    derive_forced_withdraw_message_buffer, derive_layer_zero_message_buffer, derive_role_manager,
    derive_twine_chain_storage,
};
use crate::ID;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::system_program;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
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
    AddRoleInTwineChain {
        address: Pubkey,
        role: RoleType,
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

#[derive(BorshDeserialize, BorshSerialize)]
struct AddRoleInTwineChainPayload {
    address: Pubkey,
    role: RoleType,
}

pub fn initialize_role_manager(chain_admin: &Pubkey) -> Vec<Instruction> {
    println!("Inside role manager instruction");
    let payload = TwineChainInstruction::InitializeRoleManager;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn initialize_twine_chain_storage(chain_admin: &Pubkey) -> Vec<Instruction> {
    println!("Inside Twine Chain Storage instruction");
    let payload = TwineChainInstruction::InitializeTwineChainStorage;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn initialize_message_buffer(chain_admin: &Pubkey) -> Vec<Instruction> {
    let payload = TwineChainInstruction::InitializeMessageBuffer;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_deposit_message_buffer(&ID).0, false),
        AccountMeta::new(derive_forced_withdraw_message_buffer(&ID).0, false),
        AccountMeta::new(derive_layer_zero_message_buffer(&ID).0, false),
        AccountMeta::new(derive_execution_message_buffer(&ID).0, false),
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn add_role_in_twine_chain(
    chain_admin: &Pubkey,
    address: &Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    println!("Inside add role instruction");
    let payload = TwineChainInstruction::AddRoleInTwineChain{
        address: *address,
        role: role,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    
    let accounts = vec![
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn initialize_genesis_batch(
    twine_operation_handler: &Pubkey,
    genesis_block_hash: [u8; 32],
) -> Vec<Instruction> {
    println!("Inside initialize genesis batch instruction");
    let payload = TwineChainInstruction::InitializeGenesisBatch {
        genesis_block_hash: genesis_block_hash,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(derive_commitment_pda(&ID, 0, 0).0, false),
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*twine_operation_handler, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
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
            12 => {
                println!("Checking inside 12");
                let payload = AddRoleInTwineChainPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                println!("DESERIALIZATION?");
                Ok(Self::AddRoleInTwineChain {
                    address: payload.address,
                    role: payload.role,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
