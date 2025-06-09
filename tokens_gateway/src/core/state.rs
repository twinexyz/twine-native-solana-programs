use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

/****************
 * Role Manager *
 ****************/

/// Role manager account for tokens_gateway.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TokensGatewayRoleManager {
    pub is_initialized: bool,
    pub chain_admin: Pubkey,
    pub roles: Vec<(Pubkey, RoleType)>,
}

/// Role types for authorization.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub enum RoleType {
    /// For operations such as deposits or withdrawals.
    TwineOperationHandler,
}
/**************
 * Vault Data *
 **************/
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct NativeTokenVaultData {
    pub is_initialized: bool,
    pub total_deposits: u64,
}

/// Account to hold SPL tokens vault data.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct SplTokensVaultData {
    pub is_initialized: bool,
    pub total_deposited_amount: Vec<TokenDepositData>,
}

/*********
 * Token *
 *********/
/// Account for storing token decimal mappings.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct TokenDecimalMappings {
    pub is_initialized: bool,
    pub mappings: Vec<TokenDecimalMapping>,
}

/// Data for one token deposit.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct TokenDepositData {
    pub token_id: Pubkey,
    pub amount: u64,
}

/// Represents a mapping between L1 and L2 token decimal places.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct TokenDecimalMapping {
    pub l1_token: String,
    pub l2_token: String,
    pub l1_decimals: u8,
    pub l2_decimals: u8,
}

/***************
 * Withdrawals *
 ***************/
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ExecutedWithdrawalsBuffer {
    pub is_initialized: bool,
    pub withdrawal_nonce_lower_bound: u64,
    pub executed_withdrawal_nonces: Vec<u64>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct FinalizeInputWithdrawal {
    pub public_input: ReceiptCommitment,
    pub inclusion_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ReceiptCommitment {
    pub chain_id: u64,
    pub block_number: u64,
    pub nonce: u64,
    pub is_forced_withdrawal: u8,
    pub receipt_root: [u8; 32],
    pub l1_receiver_address: String,
    pub l1_token_address: String,
    pub l2_token_address: String,
    pub amount: String,
}

impl ExecutedWithdrawalsBuffer {
    pub const SPACE: usize = 10000;

    pub fn post_withdrawal_processing(&mut self) {
        if self.executed_withdrawal_nonces.len() > 100 {
            // Sort the vector in ascending order
            self.executed_withdrawal_nonces.sort();

            let mut last_removed_nonce = self.withdrawal_nonce_lower_bound;
            let mut consecutive_nonce_count = 0;

            for nonces in self.executed_withdrawal_nonces.clone() {
                if nonces == last_removed_nonce + 1 {
                    last_removed_nonce = nonces;
                    self.withdrawal_nonce_lower_bound = nonces;
                    consecutive_nonce_count += 1;
                }
            }
            self.executed_withdrawal_nonces
                .drain(0..consecutive_nonce_count);
        }
    }
}
/*******************************************************
 *Implementation of methods for ReceiptCommitment *
 *******************************************************/
impl ReceiptCommitment {
    /// ABI encodes the receipt commitment
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::new();

        encoded.extend(&self.chain_id.to_be_bytes());
        encoded.extend(&self.block_number.to_be_bytes());
        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.receipt_root);
        encoded.extend(self.l1_receiver_address.as_bytes());
        encoded.extend(self.l1_token_address.as_bytes());
        encoded.extend(self.amount.as_bytes());

        encoded
    }
}

/******************************************************
* Implementations of IsInitialized function for PDAs *
 ******************************************************/
impl IsInitialized for TokensGatewayRoleManager {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for NativeTokenVaultData {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for SplTokensVaultData {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl IsInitialized for TokenDecimalMappings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for ExecutedWithdrawalsBuffer {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
