pub const CHAIN_ID: u64 = 900;
pub const MAX_ROLES: usize = 10;
pub const MAX_TOKENS: usize = 100;
pub const DISCRIMINATOR: usize = 8;

pub const SPL_AUTH_PREFIX: &str = "spl_auth_vault";
pub const SPL_TOKENS_VAULT_DATA_PREFIX: &str = "spl_data";
pub const DEPOSIT_TRANSACTION: &str = "deposit_transaction";
pub const ROLE_MANAGER_PREFIX: &str = "role_manager_storage";
pub const NATIVE_TOKEN_VAULT_PREFIX: &str = "native_token_vault";
pub const EXECUTED_PAYOUTS_PREFIX: &str = "executed_payouts_pda";
pub const DEPOSIT_BUFFER_PREFIX: &str = "deposit_messages_buffer";
pub const WITHDRAW_BUFFER_PREFIX: &str = "withdraw_messages_buffer";
pub const FORCED_WITHDRAW_TRANSACTION: &str = "withdraw_transaction";
pub const TOKEN_DECIMAL_MAPPINGS_PREFIX: &str = "token_mapping_buffer";
pub const EXECUTED_WITHDRAWALS_PREFIX: &str = "executed_withdrawals_pda";
pub const NATIVE_TOKEN_VAULT_DATA_PREFIX: &str = "native_token_vault_data";


//Account Sizes
pub const ROLE_MANAGER_ACCOUNT_SIZE: usize = 1 + 32 + 4 + (MAX_ROLES * 33);

pub const INITIAL_CHAIN_ADMIN: &str = "BdhpXtonNKnVKpEK7iSzZvVU1gKSWtMjUaTuQZ4rvJkS";
