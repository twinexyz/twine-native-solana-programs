use std::vec;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info,
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

use crate::{
    core::state::{MessageInfo, RoleType},
    utils::address_derivation::{
        derive_commitment_pda, derive_detailed_messages_buffer, derive_layer_zero_info,
        derive_messages_buffer, derive_messages_replicator, derive_twine_chain_role_manager,
        derive_twine_chain_storage,
    },
    ID,
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
    InitializeLayerZeroInfo,
    AppendLzDepositMessage {
        deposit_info: MessageInfo,
    },
    AppendLzForcedWithdrawalMessage {
        withdraw_info: MessageInfo,
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

pub fn initialize_twine_chain_role_manager(chain_admin: &Pubkey) -> Vec<Instruction> {
    let payload = TwineChainInstruction::InitializeRoleManager;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
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
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
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
        AccountMeta::new(derive_detailed_messages_buffer(&ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
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
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
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
    start_nonce: u64,
    end_nonce: u64,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::CopyMessagesBuffer;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_detailed_messages_buffer(&ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
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
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
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
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn remove_role_in_twine_chain(
    chain_admin: &Pubkey,
    address: &Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let payload = TwineChainInstruction::RemoveRoleInTwineChain {
        address: *address,
        role: role,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
    ];
    vec![Instruction {
        program_id: ID,
        accounts,
        data,
    }]
}

pub fn initialize_layer_zero_info(chain_admin: &Pubkey) -> Vec<Instruction> {
    let payload = TwineChainInstruction::InitializeLayerZeroInfo;

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(derive_layer_zero_info(&ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&ID).0, false),
        AccountMeta::new(*chain_admin, true),
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
                let payload = RemoveRoleInTwineChainPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveRoleInTwineChain {
                    address: payload.address,
                    role: payload.role,
                })
            }
            11 => {
                let payload = AppendDepositMessagePayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AppendLzDepositMessage {
                    deposit_info: payload.deposit_info,
                })
            }
            12 => {
                let payload = AppendForcedWithdrawalMessage::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AppendLzForcedWithdrawalMessage {
                    withdraw_info: payload.withdraw_info,
                })
            }
            13 => Ok(Self::InitializeLayerZeroInfo),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
