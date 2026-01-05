#![allow(dead_code)]
use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use spl_token;
use tokens_gateway::{
    core::{instruction::GatewayInstruction,state::RoleType},
    utils::{
        address_derivation::{
            derive_executed_payouts_pda, derive_executed_withdrawals_pda,
            derive_gateway_role_manager, derive_native_token_vault,
            derive_native_token_vault_data, derive_spl_tokens_vault_data,
            derive_spl_vault_authority, derive_token_decimal_mappings,
        },
        batch_range_provider::batch_range_provider,
    },
    ID as TOKENS_GATEWAY_ID,
};
use twine_chain::{
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_messages_buffer, derive_messages_replicator,
            derive_twine_chain_role_manager, derive_twine_chain_storage,
        },
    },
    ID as TWINE_CHAIN_ID,
};

pub fn initialize_tokens_gateway_role_manager(chain_admin: &Pubkey) -> Vec<Instruction> {
    let data = GatewayInstruction::InitializeTokensGatewayRoleManager
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(derive_gateway_role_manager(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn initialize_tokens_gateway(chain_admin: &Pubkey) -> Vec<Instruction> {
    let data = GatewayInstruction::InitializeTokensGateway.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(derive_native_token_vault(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_gateway_role_manager(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn update_gateway_token_mapping(
    l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,
    chain_admin: &Pubkey,
) -> Vec<Instruction> {
    let data = GatewayInstruction::UpdateTokenMapping {
        l1_token,
        l2_token,
        l1_decimals,
        l2_decimals,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_gateway_role_manager(&TOKENS_GATEWAY_ID).0, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn remove_gateway_token_mapping(
    l1_token: String,
    l2_token: String,
    chain_admin: &Pubkey,
) -> Vec<Instruction> {
    let data = GatewayInstruction::RemoveTokenMapping { l1_token, l2_token }
        .try_to_vec()
        .unwrap();

    let accounts = vec![
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_gateway_role_manager(&TOKENS_GATEWAY_ID).0, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn native_token_deposit(
    user: &Pubkey,
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    start_nonce: u64,
    end_nonce: u64,
    data: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::NativeTokenDeposit {
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        data,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(derive_native_token_vault(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn spl_token_deposit(
    user: &Pubkey,
    user_token_account: &Pubkey,
    token_mint_pubkey: &Pubkey,
    spl_tokens_vault: &Pubkey,
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    start_nonce: u64,
    end_nonce: u64,
    data: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::SplTokenDeposit {
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        data,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn forced_native_token_withdrawal(
    user: &Pubkey,
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    start_nonce: u64,
    end_nonce: u64,
    signature: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::NativeTokenForcedWithdrawal {
        from_twine_address,
        to_l1_pubkey,
        l1_token,
        l2_token,
        amount,
        signature,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(derive_native_token_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn forced_spl_token_withdrawal(
    user: &Pubkey,
    to_token_account: &Pubkey,
    token_mint_pubkey: &Pubkey,
    from_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    start_nonce: u64,
    end_nonce: u64,
    signature: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::SplTokenForcedWithdrawal {
        from_twine_address,
        to_l1_pubkey: to_token_account.to_string(),
        l1_token,
        l2_token,
        amount,
        signature,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*to_token_account, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn execute_l2_native_withdrawal(
    initializer: &Pubkey,
    l1_receiver_address: Pubkey,
    message_nonce: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::ExecuteL2NativeWithdrawal {
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_native_token_vault(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_executed_withdrawals_pda(&TOKENS_GATEWAY_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new_readonly(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new_readonly(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn execute_l2_spl_withdrawal(
    initializer: &Pubkey,
    token_mint_pubkey: &Pubkey,
    spl_tokens_vault: &Pubkey,
    l1_receiver_address: Pubkey,
    message_nonce: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::ExecuteL2SplWithdrawal {
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_spl_tokens_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(derive_spl_vault_authority(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_executed_withdrawals_pda(&TOKENS_GATEWAY_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new_readonly(derive_twine_chain_role_manager(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new_readonly(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn process_native_refund(
    initializer: &Pubkey,
    l1_receiver_address: Pubkey,
    message_nonce: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::ProcessNativeRefund {
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_native_token_vault(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&TOKENS_GATEWAY_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn process_spl_refund(
    initializer: &Pubkey,
    token_mint_pubkey: &Pubkey,
    spl_tokens_vault: &Pubkey,
    l1_receiver_address: Pubkey,
    message_nonce: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::ProcessSplRefund {
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_spl_tokens_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(derive_spl_vault_authority(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&TOKENS_GATEWAY_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn process_native_forced_withdrawal(
    initializer: &Pubkey,
    l1_receiver_address: Pubkey,
    message_nonce: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::ProcessNativeForcedWithdrawal {
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_native_token_vault(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&TOKENS_GATEWAY_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new_readonly(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn process_spl_forced_withdrawal(
    initializer: &Pubkey,
    token_mint_pubkey: &Pubkey,
    spl_tokens_vault: &Pubkey,
    l1_receiver_address: Pubkey,
    message_nonce: u64,
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
) -> Vec<Instruction> {
    let data = GatewayInstruction::ProcessSplForcedWithdrawal {
        public_values,
        execution_proof,
    }
    .try_to_vec()
    .unwrap();

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_spl_tokens_vault_data(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(derive_spl_vault_authority(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_twine_chain_storage(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&TOKENS_GATEWAY_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new(derive_token_decimal_mappings(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&TWINE_CHAIN_ID).0, false),
        AccountMeta::new(
            derive_messages_replicator(&TWINE_CHAIN_ID, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(TWINE_CHAIN_ID, false),
    ];

    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn set_gateway_role_chain_admin(new_admin: Pubkey, chain_admin: Pubkey) -> Vec<Instruction> {
    let data = GatewayInstruction::SetGatewayRoleChainAdmin { new_admin }
        .try_to_vec()
        .unwrap();

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(chain_admin, true),
    ];
    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn add_role_in_gateway(
    chain_admin: Pubkey,
    account_address: Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let data = GatewayInstruction::AddRoleInGateway {
        address: account_address,
        role,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(chain_admin, true),
    ];
    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}

pub fn remove_role_in_gateway(
    chain_admin: Pubkey,
    account_address: Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let data = GatewayInstruction::RemoveRoleInGateway {
        address: account_address,
        role,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&TOKENS_GATEWAY_ID).0, false),
        AccountMeta::new(chain_admin, true),
    ];
    vec![Instruction {
        program_id: TOKENS_GATEWAY_ID,
        accounts,
        data,
    }]
}
