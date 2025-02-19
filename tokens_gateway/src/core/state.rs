use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Role manager account for tokens_gateway.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TokensGatewayRoleManager {
    pub chain_admin: Pubkey,
    pub roles: Vec<(Pubkey, RoleType)>,
}

/// Role types for authorization.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub enum RoleType {
    /// For operations such as deposits or withdrawals.
    TwineOperationHandler,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct NativeTokenVaultData {
    pub total_deposits: u64,
}

/// Account to hold SPL tokens vault data.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct SplTokensVault {
    pub authority: Pubkey,
    pub total_deposited_amount: Vec<TokenDepositData>,
}

/// Data for one token deposit.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct TokenDepositData {
    pub token_id: Pubkey,
    pub amount: u64,
}

/// Account for storing token decimal mappings.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct TokenDecimalMappings {
    pub authority: Pubkey,
    pub mappings: Vec<TokenDecimalMapping>,
}

/// Represents a mapping between L1 and L2 token decimal places.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct TokenDecimalMapping {
    pub l1_token: String,
    pub l2_token: String,
    pub l1_decimals: u8,
    pub l2_decimals: u8,
}
