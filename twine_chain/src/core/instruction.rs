use std::vec;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},program_error::ProgramError, pubkey::Pubkey, system_program
};

use crate::{
    core::state::{DepositMessageInfo, ForcedWithdrawMessageInfo, RoleType},
    utils::address_derivation::{
        derive_commitment_pda, derive_execution_message_buffer, derive_layer_zero_message_buffer,
        derive_messages_buffer, derive_role_manager, derive_twine_chain_storage,derive_messages_replicator
    },
    ID,
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
        genesis_batch_hash: [u8; 32],
    },
    CommitBatch {
        batch_number: u64,
        batch_hash: [u8; 32],
    },
    FinalizeBatch {
        batch_number: u64,
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
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
struct CommitBatchPayload {
    batch_number: u64,
    batch_hash: [u8; 32],
}

#[derive(BorshDeserialize)]
struct FinalizeBatchPayload {
    batch_number: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
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

pub fn initialize_twine_chain_role_manager(chain_admin: &Pubkey) -> Vec<Instruction> {
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
        AccountMeta::new(derive_messages_buffer(&ID).0, false),
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

pub fn append_deposit_message(
    twine_operation_handler: &Pubkey,
    deposit_info: DepositMessageInfo,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::AppendDepositMessage { deposit_info };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_messages_buffer(&ID).0, false),
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*twine_operation_handler, true),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn append_forced_withdrawal_message(
    twine_operation_handler: &Pubkey,
    withdraw_info: ForcedWithdrawMessageInfo,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::AppendForcedWithdrawalMessage { withdraw_info };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_messages_buffer(&ID).0, false),
        AccountMeta::new(derive_role_manager(&ID).0, false),
        AccountMeta::new(*twine_operation_handler, true),
    ];

    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn initialize_genesis_batch(
    twine_operation_handler: &Pubkey,
    genesis_batch_hash: [u8; 32],
) -> Vec<Instruction> {
    let payload: TwineChainInstruction =
        TwineChainInstruction::InitializeGenesisBatch { genesis_batch_hash };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(derive_commitment_pda(&ID, 0).0, false),
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
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

pub fn commit_batch(
    twine_operation_handler: &Pubkey,
    batch_number: u64,
    batch_hash: [u8; 32],
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::CommitBatch {
        batch_number,
        batch_hash,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
        AccountMeta::new(derive_commitment_pda(&ID, batch_number).0, false),
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

pub fn finalize_batch(
    twine_operation_handler: &Pubkey,
    batch_number: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::FinalizeBatch {
        batch_number,
        public_values,
        execution_proof,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
        AccountMeta::new(derive_commitment_pda(&ID, batch_number).0, false),
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

pub fn copy_messages_buffer(
    twine_operation_handler: &Pubkey,
    start_nonce:u64,
    end_nonce:u64
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::CopyMessagesBuffer;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_messages_buffer(&ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&ID, start_nonce, end_nonce).0,
            false,
        ),
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
pub fn commit_and_finalize_batch(
    twine_operation_handler: &Pubkey,
    batch_number: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::CommitAndFinalizeBatch {
        batch_number,
        public_values,
        execution_proof,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
        AccountMeta::new(derive_commitment_pda(&ID, batch_number).0, false),
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

pub fn add_role_in_twine_chain(
    chain_admin: &Pubkey,
    address: &Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::AddRoleInTwineChain {
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
                    genesis_batch_hash: payload.genesis_block_hash,
                })
            }
            8 => {
                let payload = CommitBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::CommitBatch {
                    batch_number: payload.batch_number,
                    batch_hash: payload.batch_hash,
                })
            }
            9 => {
                let payload = FinalizeBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::FinalizeBatch {
                    batch_number: payload.batch_number,
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            10 => {
                let payload = AddRoleInTwineChainPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AddRoleInTwineChain {
                    address: payload.address,
                    role: payload.role,
                })
            }
            11 => Ok(Self::CopyMessagesBuffer),
            12 => {
                let payload = CommitAndFinalizeBatchPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::CommitAndFinalizeBatch {
                    batch_number: payload.batch_number,
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
