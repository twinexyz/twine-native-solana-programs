#![allow(dead_code)]
use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use twine_chain::{
    core::{
        instruction::TwineChainInstruction,
        state::{MessageInfo, RoleType},
    },
    utils::address_derivation::{
        derive_commitment_pda, derive_detailed_messages_buffer, derive_messages_buffer,
        derive_messages_replicator, derive_twine_chain_role_manager, derive_twine_chain_storage,
    },
    ID as TWINE_CHAIN_ID,
};

pub fn initialize_twine_chain_role_manager(chain_admin: &Pubkey) -> Vec<Instruction> {
    let data = TwineChainInstruction::InitializeRoleManager
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn initialize_twine_chain_storage(chain_admin: &Pubkey) -> Vec<Instruction> {
    let data = TwineChainInstruction::InitializeTwineChainStorage
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn initialize_message_buffer(chain_admin: &Pubkey) -> Vec<Instruction> {
    let data = TwineChainInstruction::InitializeMessageBuffer
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn set_vkeys(
    twine_operation_handler: &Pubkey,
    groth16_vk: Vec<u8>,
    finalize_vkey: String,
    refund_vkey: String,
    forced_withdrawal_vkey: String,
    l2_withdrawal_vkey: String,
) -> Vec<Instruction> {
    let data = TwineChainInstruction::SetVkeys {
        groth16_vk,
        finalize_vkey,
        refund_vkey,
        forced_withdrawal_vkey,
        l2_withdrawal_vkey,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*twine_operation_handler, true),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn append_deposit_message(
    initializer: &Pubkey,
    deposit_info: MessageInfo,
) -> Vec<Instruction> {
    let data = TwineChainInstruction::AppendDepositMessage { deposit_info }
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*initializer, true),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn append_forced_withdrawal_message(
    initializer: &Pubkey,
    withdraw_info: MessageInfo,
) -> Vec<Instruction> {
    let data =
        TwineChainInstruction::AppendForcedWithdrawalMessage { withdraw_info }
            .try_to_vec()
            .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*initializer, true),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn initialize_genesis_batch(
    twine_operation_handler: &Pubkey,
    genesis_batch_hash: [u8; 32],
) -> Vec<Instruction> {
    let data = TwineChainInstruction::InitializeGenesisBatch { genesis_batch_hash }
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_commitment_pda(&TWINE_CHAIN_ID, 0).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*twine_operation_handler, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn copy_messages_buffer(
    twine_operation_handler: &Pubkey,
    start_nonce: u64,
    end_nonce: u64,
) -> Vec<Instruction> {
    let data = TwineChainInstruction::CopyMessagesBuffer
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new(*twine_operation_handler, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
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
    let data = TwineChainInstruction::CommitAndFinalizeBatch {
        batch_number,
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_commitment_pda(&TWINE_CHAIN_ID, batch_number).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*twine_operation_handler, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn add_role_in_twine_chain(
    chain_admin: &Pubkey,
    address: &Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let data = TwineChainInstruction::AddRoleInTwineChain {
        address: *address,
        role,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*chain_admin, true),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}

pub fn remove_role_in_twine_chain(
    chain_admin: &Pubkey,
    address: &Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let data = TwineChainInstruction::RemoveRoleInTwineChain {
        address: *address,
        role,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(*chain_admin, true),
    ];
    vec![Instruction {
        program_id: TWINE_CHAIN_ID,
        accounts,
        data,
    }]
}
