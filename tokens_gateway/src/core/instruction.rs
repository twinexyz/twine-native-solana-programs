use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use oapp::{
    core::state::{DVN_CONFIG_SEED, EXECUTOR_CONFIG_SEED},
    utils::address_derivation::*,
    ID as oapp_program_id,
};
use solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};
use twine_chain::{
    core::state::TwineChainStorage,
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_execution_message_buffer,
            derive_layer_zero_info, derive_messages_buffer, derive_messages_replicator,
            derive_twine_chain_role_manager, derive_twine_chain_storage,
        },
        constants::MESSAGE_NONCE_GAP,
    },
    ID as twine_chain_id,
};

use super::state::RoleType;

use crate::{
    core::state::LzMessageParams,
    utils::{
        address_derivation::{
            derive_executed_payouts_pda, derive_executed_withdrawals_pda,
            derive_gateway_role_manager, derive_native_token_vault, derive_native_token_vault_data,
            derive_spl_tokens_vault_data, derive_spl_vault_authority,
            derive_token_decimal_mappings,
        },
        batch_range_provider::batch_range_provider,
    },
    ID as tokens_gateway_ID,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum GatewayInstruction {
    InitializeTokensGatewayRoleManager,
    InitializeTokensGateway,
    UpdateTokenMapping {
        l1_token: String,
        l2_token: String,
        l1_decimals: u8,
        l2_decimals: u8,
    },
    SetGatewayRoleChainAdmin {
        new_admin: Pubkey,
    },
    AddRoleInGateway {
        address: Pubkey,
        role: RoleType,
    },
    RemoveRoleInGateway {
        address: Pubkey,
        role: RoleType,
    },
    NativeTokenDeposit {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        data: Vec<u8>,
    },
    SplTokenDeposit {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        data: Vec<u8>,
    },
    NativeTokenForcedWithdrawal {
        from_twine_address: String,
        to_l1_pubkey: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        signature: Vec<u8>,
    },
    SplTokenForcedWithdrawal {
        from_twine_address: String,
        to_l1_pubkey: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        signature: Vec<u8>,
    },
    ExecuteL2NativeWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ExecuteL2SplWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessNativeRefund {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessSplRefund {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessNativeForcedWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessSplForcedWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    RemoveTokenMapping {
        l1_token: String,
        l2_token: String,
    },
    LzNativeTokenDeposit {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        data: Vec<u8>,
    },
    LzSplTokenDeposit {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        data: Vec<u8>,
    },
    LzNativeTokenForcedWithdrawal {
        from_twine_address: String,
        to_l1_pubkey: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        signature: Vec<u8>,
    },
    LzSplTokenForcedWithdrawal {
        from_twine_address: String,
        to_l1_pubkey: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        signature: Vec<u8>,
    },
}
#[derive(BorshSerialize, BorshDeserialize)]
struct UpdateTokenMappingPayload {
    l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RemoveTokenMappingPayload {
    l1_token: String,
    l2_token: String,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct SetGatewayRoleChainAdminPayload {
    new_admin: Pubkey,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct AddRoleInGatewayPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RemoveRoleInGatewayPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct NativeTokenDepositPayload {
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct SplTokenDepositPayload {
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct NativeTokenForcedWithdrawalPayload {
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct SplTokenForcedWithdrawalPayload {
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ExecuteL2NativeWithdrawalPayload {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ExecuteL2SplWithdrawalPayload {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessNativeRefund {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessSplRefund {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessNativeForcedWithdrawal {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessSplForcedWithdrawal {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

pub fn initialize_tokens_gateway_role_manager(chain_admin: &Pubkey) -> Vec<Instruction> {
    let payload = GatewayInstruction::InitializeTokensGatewayRoleManager;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_gateway_role_manager(&tokens_gateway_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];
    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn initialize_tokens_gateway(chain_admin: &Pubkey) -> Vec<Instruction> {
    let payload = GatewayInstruction::InitializeTokensGateway;
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());
    let accounts = vec![
        AccountMeta::new(derive_native_token_vault(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_gateway_role_manager(&tokens_gateway_ID).0, false),
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(system_program::id(), false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::UpdateTokenMapping {
        l1_token,
        l2_token,
        l1_decimals,
        l2_decimals,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_gateway_role_manager(&tokens_gateway_ID).0, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn remove_gateway_token_mapping(
    l1_token: String,
    l2_token: String,
    chain_admin: &Pubkey,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::RemoveTokenMapping { l1_token, l2_token };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*chain_admin, true),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_gateway_role_manager(&tokens_gateway_ID).0, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::NativeTokenDeposit {
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        data,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(derive_native_token_vault(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn lz_native_token_deposit(
    user: &Pubkey,
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    start_nonce: u64,
    end_nonce: u64,
    data: Vec<u8>,
    // Layer Zero Requirements
    dvn_program: &Pubkey,
    executor_program: &Pubkey,
    params: LzMessageParams,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::LzNativeTokenDeposit {
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        data,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = derive_store_pda(&oapp_program_id).0;
    let native_loader_program_id =
        Pubkey::from_str("NativeLoader1111111111111111111111111111111").unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(derive_native_token_vault(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
        AccountMeta::new(derive_layer_zero_info(&twine_chain_id).0, false),
        // <------------------- Endpoint Accounts --------------------------->
        // sender
        AccountMeta::new(store_account, false),
        // sendLibraryProgram (ULN Program)
        AccountMeta::new_readonly(get_send_library_program(), false),
        // sendLibraryConfig
        AccountMeta::new(
            derive_send_library_config(&store_account, &params.dst_eid).0,
            false,
        ),
        // defaultSendLibraryConfig
        AccountMeta::new(derive_default_send_library_config(&params.dst_eid).0, false),
        // sendLibraryInfo (sendLibrary: 2Xg...LkQ)
        AccountMeta::new_readonly(derive_send_library_info().0, false),
        // endpointSettings
        AccountMeta::new(derive_endpoint_settings().0, false),
        // nonce
        AccountMeta::new(
            derive_nonce(&store_account, &params.dst_eid, &params.receiver).0,
            false,
        ),
        // eventAuthority
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_endpoint_id(), false),
        // <------------------- Library Accounts --------------------------->
        // uln
        AccountMeta::new(derive_uln().0, false),
        // sendConfig
        AccountMeta::new(derive_send_config(&params.dst_eid, &store_account).0, false),
        // defaultSendConfig
        AccountMeta::new(derive_default_send_config(&params.dst_eid).0, false),
        // payer
        AccountMeta::new(*user, true),
        // treasury (Optional)
        AccountMeta::new(*user, false),
        // systemProgram
        AccountMeta::new_readonly(system_program::ID, false),
        // eventAuthority
        AccountMeta::new(derive_library_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_send_library_program(), false),
        // <------------------ Remaining Accounts ------------------------->
        // Executor Program
        AccountMeta::new_readonly(*executor_program, false),
        // Executor Config
        AccountMeta::new(
            Pubkey::find_program_address(&[EXECUTOR_CONFIG_SEED], executor_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
        // DVN program
        AccountMeta::new_readonly(*dvn_program, false),
        // dvn config
        AccountMeta::new(
            Pubkey::find_program_address(&[DVN_CONFIG_SEED], dvn_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::NativeTokenForcedWithdrawal {
        from_twine_address,
        to_l1_pubkey,
        l1_token,
        l2_token,
        amount,
        signature,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn lz_forced_native_token_withdrawal(
    user: &Pubkey,
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    start_nonce: u64,
    end_nonce: u64,
    signature: Vec<u8>,
    // Layer Zero Requirements
    dvn_program: &Pubkey,
    executor_program: &Pubkey,
    params: LzMessageParams,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::LzNativeTokenForcedWithdrawal {
        from_twine_address,
        to_l1_pubkey,
        l1_token,
        l2_token,
        amount,
        signature,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = derive_store_pda(&oapp_program_id).0;
    let native_loader_program_id =
        Pubkey::from_str("NativeLoader1111111111111111111111111111111").unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
        AccountMeta::new(derive_layer_zero_info(&twine_chain_id).0, false),
        // <------------------- Endpoint Accounts --------------------------->
        // sender
        AccountMeta::new(store_account, false),
        // sendLibraryProgram (ULN Program)
        AccountMeta::new_readonly(get_send_library_program(), false),
        // sendLibraryConfig
        AccountMeta::new(
            derive_send_library_config(&store_account, &params.dst_eid).0,
            false,
        ),
        // defaultSendLibraryConfig
        AccountMeta::new(derive_default_send_library_config(&params.dst_eid).0, false),
        // sendLibraryInfo (sendLibrary: 2Xg...LkQ)
        AccountMeta::new_readonly(derive_send_library_info().0, false),
        // endpointSettings
        AccountMeta::new(derive_endpoint_settings().0, false),
        // nonce
        AccountMeta::new(
            derive_nonce(&store_account, &params.dst_eid, &params.receiver).0,
            false,
        ),
        // eventAuthority
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_endpoint_id(), false),
        // <------------------- Library Accounts --------------------------->
        // uln
        AccountMeta::new(derive_uln().0, false),
        // sendConfig
        AccountMeta::new(derive_send_config(&params.dst_eid, &store_account).0, false),
        // defaultSendConfig
        AccountMeta::new(derive_default_send_config(&params.dst_eid).0, false),
        // payer
        AccountMeta::new(*user, true),
        // treasury (Optional)
        AccountMeta::new(*user, false),
        // systemProgram
        AccountMeta::new_readonly(system_program::ID, false),
        // eventAuthority
        AccountMeta::new(derive_library_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_send_library_program(), false),
        // <------------------ Remaining Accounts ------------------------->
        // Executor Program
        AccountMeta::new_readonly(*executor_program, false),
        // Executor Config
        AccountMeta::new(
            Pubkey::find_program_address(&[EXECUTOR_CONFIG_SEED], executor_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
        // DVN program
        AccountMeta::new_readonly(*dvn_program, false),
        // dvn config
        AccountMeta::new(
            Pubkey::find_program_address(&[DVN_CONFIG_SEED], dvn_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::SplTokenDeposit {
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        data,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn lz_spl_token_deposit(
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
    // Layer Zero Requirements
    dvn_program: &Pubkey,
    executor_program: &Pubkey,
    params: LzMessageParams,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::LzSplTokenDeposit {
        receiver_twine_address,
        l1_token,
        l2_token,
        amount,
        data,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = derive_store_pda(&oapp_program_id).0;
    let native_loader_program_id =
        Pubkey::from_str("NativeLoader1111111111111111111111111111111").unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
        AccountMeta::new(derive_layer_zero_info(&twine_chain_id).0, false),
        // <------------------- Endpoint Accounts --------------------------->
        // sender
        AccountMeta::new(store_account, false),
        // sendLibraryProgram (ULN Program)
        AccountMeta::new_readonly(get_send_library_program(), false),
        // sendLibraryConfig
        AccountMeta::new(
            derive_send_library_config(&store_account, &params.dst_eid).0,
            false,
        ),
        // defaultSendLibraryConfig
        AccountMeta::new(derive_default_send_library_config(&params.dst_eid).0, false),
        // sendLibraryInfo (sendLibrary: 2Xg...LkQ)
        AccountMeta::new_readonly(derive_send_library_info().0, false),
        // endpointSettings
        AccountMeta::new(derive_endpoint_settings().0, false),
        // nonce
        AccountMeta::new(
            derive_nonce(&store_account, &params.dst_eid, &params.receiver).0,
            false,
        ),
        // eventAuthority
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_endpoint_id(), false),
        // <------------------- Library Accounts --------------------------->
        // uln
        AccountMeta::new(derive_uln().0, false),
        // sendConfig
        AccountMeta::new(derive_send_config(&params.dst_eid, &store_account).0, false),
        // defaultSendConfig
        AccountMeta::new(derive_default_send_config(&params.dst_eid).0, false),
        // payer
        AccountMeta::new(*user, true),
        // treasury (Optional)
        AccountMeta::new(*user, false),
        // systemProgram
        AccountMeta::new_readonly(system_program::ID, false),
        // eventAuthority
        AccountMeta::new(derive_library_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_send_library_program(), false),
        // <------------------ Remaining Accounts ------------------------->
        // Executor Program
        AccountMeta::new_readonly(*executor_program, false),
        // Executor Config
        AccountMeta::new(
            Pubkey::find_program_address(&[EXECUTOR_CONFIG_SEED], executor_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
        // DVN program
        AccountMeta::new_readonly(*dvn_program, false),
        // dvn config
        AccountMeta::new(
            Pubkey::find_program_address(&[DVN_CONFIG_SEED], dvn_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::SplTokenForcedWithdrawal {
        from_twine_address,
        to_l1_pubkey: to_token_account.to_string(),
        l1_token,
        l2_token,
        amount,
        signature,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*to_token_account, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn lz_forced_spl_token_withdrawal(
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
    // Layer Zero Requirements
    dvn_program: &Pubkey,
    executor_program: &Pubkey,
    params: LzMessageParams,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::SplTokenForcedWithdrawal {
        from_twine_address,
        to_l1_pubkey: to_token_account.to_string(),
        l1_token,
        l2_token,
        amount,
        signature,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let store_account = derive_store_pda(&oapp_program_id).0;
    let native_loader_program_id =
        Pubkey::from_str("NativeLoader1111111111111111111111111111111").unwrap();

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(*to_token_account, false),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
        AccountMeta::new(derive_layer_zero_info(&twine_chain_id).0, false),
        // <------------------- Endpoint Accounts --------------------------->
        // sender
        AccountMeta::new(store_account, false),
        // sendLibraryProgram (ULN Program)
        AccountMeta::new_readonly(get_send_library_program(), false),
        // sendLibraryConfig
        AccountMeta::new(
            derive_send_library_config(&store_account, &params.dst_eid).0,
            false,
        ),
        // defaultSendLibraryConfig
        AccountMeta::new(derive_default_send_library_config(&params.dst_eid).0, false),
        // sendLibraryInfo (sendLibrary: 2Xg...LkQ)
        AccountMeta::new_readonly(derive_send_library_info().0, false),
        // endpointSettings
        AccountMeta::new(derive_endpoint_settings().0, false),
        // nonce
        AccountMeta::new(
            derive_nonce(&store_account, &params.dst_eid, &params.receiver).0,
            false,
        ),
        // eventAuthority
        AccountMeta::new(derive_endpoint_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_endpoint_id(), false),
        // <------------------- Library Accounts --------------------------->
        // uln
        AccountMeta::new(derive_uln().0, false),
        // sendConfig
        AccountMeta::new(derive_send_config(&params.dst_eid, &store_account).0, false),
        // defaultSendConfig
        AccountMeta::new(derive_default_send_config(&params.dst_eid).0, false),
        // payer
        AccountMeta::new(*user, true),
        // treasury (Optional)
        AccountMeta::new(*user, false),
        // systemProgram
        AccountMeta::new_readonly(system_program::ID, false),
        // eventAuthority
        AccountMeta::new(derive_library_event_authority().0, false),
        // program
        AccountMeta::new_readonly(get_send_library_program(), false),
        // <------------------ Remaining Accounts ------------------------->
        // Executor Program
        AccountMeta::new_readonly(*executor_program, false),
        // Executor Config
        AccountMeta::new(
            Pubkey::find_program_address(&[EXECUTOR_CONFIG_SEED], executor_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
        // DVN program
        AccountMeta::new_readonly(*dvn_program, false),
        // dvn config
        AccountMeta::new(
            Pubkey::find_program_address(&[DVN_CONFIG_SEED], dvn_program).0,
            false,
        ),
        // Price feed Program
        AccountMeta::new_readonly(native_loader_program_id, false),
        // Price feed config
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::ExecuteL2NativeWithdrawal {
        public_values: public_values,
        execution_proof: execution_proof,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_native_token_vault(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_executed_withdrawals_pda(&tokens_gateway_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new_readonly(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new_readonly(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::ExecuteL2SplWithdrawal {
        public_values: public_values,
        execution_proof: execution_proof,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(derive_spl_vault_authority(&tokens_gateway_ID).0, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_executed_withdrawals_pda(&tokens_gateway_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new_readonly(derive_twine_chain_role_manager(&twine_chain_id).0, false),
        AccountMeta::new_readonly(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::ProcessNativeRefund {
        public_values: public_values,
        execution_proof: execution_proof,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_native_token_vault(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&tokens_gateway_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::ProcessSplRefund {
        public_values: public_values,
        execution_proof: execution_proof,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(derive_spl_vault_authority(&tokens_gateway_ID).0, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&tokens_gateway_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::ProcessNativeForcedWithdrawal {
        public_values: public_values,
        execution_proof: execution_proof,
    };
    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_native_token_vault(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_native_token_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&tokens_gateway_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new_readonly(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
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
    let payload = GatewayInstruction::ProcessSplForcedWithdrawal {
        public_values: public_values,
        execution_proof: execution_proof,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let (start_nonce, end_nonce) =
        batch_range_provider(message_nonce).expect("Failed to calculate batch range");
    let accounts = vec![
        AccountMeta::new(*initializer, true),
        AccountMeta::new(derive_spl_tokens_vault_data(&tokens_gateway_ID).0, false),
        AccountMeta::new(*spl_tokens_vault, false),
        AccountMeta::new(derive_spl_vault_authority(&tokens_gateway_ID).0, false),
        AccountMeta::new(spl_token::id(), false),
        AccountMeta::new(*token_mint_pubkey, false),
        AccountMeta::new(derive_twine_chain_storage(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_executed_payouts_pda(&tokens_gateway_ID, message_nonce).0,
            false,
        ),
        AccountMeta::new(l1_receiver_address, false),
        AccountMeta::new(derive_token_decimal_mappings(&tokens_gateway_ID).0, false),
        AccountMeta::new(derive_detailed_messages_buffer(&twine_chain_id).0, false),
        AccountMeta::new(
            derive_messages_replicator(&twine_chain_id, start_nonce, end_nonce).0,
            false,
        ),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(twine_chain_id, false),
    ];

    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn set_gateway_role_chain_admin(new_admin: Pubkey, chain_admin: Pubkey) -> Vec<Instruction> {
    let payload = GatewayInstruction::SetGatewayRoleChainAdmin {
        new_admin: new_admin,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&tokens_gateway_ID).0, false),
        AccountMeta::new(chain_admin, true),
    ];
    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn add_role_in_gateway(
    chain_admin: Pubkey,
    account_address: Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::AddRoleInGateway {
        address: account_address,
        role: role,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&tokens_gateway_ID).0, false),
        AccountMeta::new(chain_admin, true),
    ];
    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

pub fn remove_role_in_gateway(
    chain_admin: Pubkey,
    account_address: Pubkey,
    role: RoleType,
) -> Vec<Instruction> {
    let payload = GatewayInstruction::RemoveRoleInGateway {
        address: account_address,
        role: role,
    };

    let mut data = vec![];
    data.extend(payload.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(derive_twine_chain_role_manager(&tokens_gateway_ID).0, false),
        AccountMeta::new(chain_admin, true),
    ];
    vec![Instruction {
        program_id: tokens_gateway_ID,
        accounts,
        data,
    }]
}

impl GatewayInstruction {
    pub fn unpack_instruction(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        match discriminator {
            0 => Ok(Self::InitializeTokensGatewayRoleManager),
            1 => Ok(Self::InitializeTokensGateway),
            2 => {
                let payload = UpdateTokenMappingPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::UpdateTokenMapping {
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    l1_decimals: payload.l1_decimals,
                    l2_decimals: payload.l2_decimals,
                })
            }
            3 => {
                let payload = SetGatewayRoleChainAdminPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetGatewayRoleChainAdmin {
                    new_admin: payload.new_admin,
                })
            }

            4 => {
                let payload = AddRoleInGatewayPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AddRoleInGateway {
                    address: payload.address,
                    role: payload.role,
                })
            }
            5 => {
                let payload = RemoveRoleInGatewayPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveRoleInGateway {
                    address: payload.address,
                    role: payload.role,
                })
            }
            6 => {
                let payload = NativeTokenDepositPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::NativeTokenDeposit {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    data: payload.data,
                })
            }

            7 => {
                let payload = SplTokenDepositPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SplTokenDeposit {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    data: payload.data,
                })
            }

            8 => {
                let payload = NativeTokenForcedWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::NativeTokenForcedWithdrawal {
                    from_twine_address: payload.from_twine_address,
                    to_l1_pubkey: payload.to_l1_pubkey,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    signature: payload.signature,
                })
            }
            9 => {
                let payload = SplTokenForcedWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SplTokenForcedWithdrawal {
                    from_twine_address: payload.from_twine_address,
                    to_l1_pubkey: payload.to_l1_pubkey,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    signature: payload.signature,
                })
            }
            10 => {
                let payload = ExecuteL2NativeWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ExecuteL2NativeWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            11 => {
                let payload = ExecuteL2SplWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ExecuteL2SplWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            12 => {
                let payload = ProcessNativeRefund::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessNativeRefund {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            13 => {
                let payload = ProcessSplRefund::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessSplRefund {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            14 => {
                let payload = ProcessNativeForcedWithdrawal::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessNativeForcedWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            15 => {
                let payload = ProcessSplForcedWithdrawal::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessSplForcedWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            16 => {
                let payload = RemoveTokenMappingPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveTokenMapping {
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                })
            }
            17 => {
                let payload = NativeTokenDepositPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::LzNativeTokenDeposit {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    data: payload.data,
                })
            }

            18 => {
                let payload = SplTokenDepositPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::LzSplTokenDeposit {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    data: payload.data,
                })
            }

            19 => {
                let payload = NativeTokenForcedWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::LzNativeTokenForcedWithdrawal {
                    from_twine_address: payload.from_twine_address,
                    to_l1_pubkey: payload.to_l1_pubkey,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    signature: payload.signature,
                })
            }
            20 => {
                let payload = SplTokenForcedWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::LzSplTokenForcedWithdrawal {
                    from_twine_address: payload.from_twine_address,
                    to_l1_pubkey: payload.to_l1_pubkey,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    signature: payload.signature,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
