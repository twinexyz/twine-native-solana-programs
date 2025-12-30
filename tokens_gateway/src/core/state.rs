use crate::core::error::ProgramCustomError;
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
use solana_program::{program_error::ProgramError, program_pack::IsInitialized, pubkey::Pubkey};
use twine_chain::core::state::TransactionType;

/****************
 * Role Manager *
 ****************/

/// Role manager account for tokens_gateway.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct TokensGatewayRoleManager {
    pub is_initialized: bool,
    pub chain_admin: Pubkey,
    pub roles: Vec<(Pubkey, RoleType)>,
}

/// Role types for authorization.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub enum RoleType {
    TwineOperationHandler,
}
/**************
 * Vault Data *
 **************/
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
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
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]

pub struct TokenDecimalMappings {
    pub is_initialized: bool,
    pub mappings: Vec<TokenDecimalMappingData>,
}

/// Data for one token deposit.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct TokenDepositData {
    pub token_id: Pubkey,
    pub amount: u64,
}

/// Represents a mapping between L1 and L2 token decimal places.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct TokenDecimalMappingData {
    pub l1_token: String,
    pub l2_token: String,
    pub l1_decimals: u8,
    pub l2_decimals: u8,
}

/***************
 * Refund *
 ***************/
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct L1OriginTxPublicValues {
    pub batch_hash: [u8; 32],
    pub batch_number: u64,
    pub txn_type: TransactionType,
    pub nonce: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub message: [u8; 32],
    pub l1_address: String,
    pub l2_address: String,
    pub l1_token_address: String,
    pub l2_token_address: String,
    pub amount: String,
}

/***************
 * Withdrawals *
 ***************/

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct L2WithdrawValues {
    pub batch_number: u64,
    pub nonce: u64,
    pub batch_hash: [u8; 32],
    pub l1_receiver_address: String,
    pub l1_token_address: String,
    pub l2_token_address: String,
    pub amount: String,
}


/// Struct for signed messageAdd commentMore actions
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct SignMessageInfo {
    pub nonce: u64,
    pub chain_id: u64,
    pub amount: u64,
    pub l1_pubkey: String,
    pub twine_address: String,
    pub l1_token: String,
    pub l2_token: String,
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

/**********
 * Events *
 *********/
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RefundSuccessfulEvent {
    pub event: String,
    pub nonce: u64,
    pub l1_receiver: String,
    pub l1_token: String,
    pub chain_id: u64,
    pub amount: u64,
    pub slot_number: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ForcedWithdrawalSuccessfulEvent {
    pub event: String,
    pub nonce: u64,
    pub l1_receiver: String,
    pub l1_token: String,
    pub chain_id: u64,
    pub amount: u64,
    pub slot_number: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct L2WithdrawExecutedEvent {
    pub event: String,
    pub nonce: u64,
    pub l1_token: String,
    pub l2_token: String,
    pub l1_receiver: String,
    pub chain_id: u64,
    pub amount: u64,
    pub slot_number: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct LzMessageParams { 
    pub dst_eid: u32,       // Destination chain ID
    pub receiver: [u8; 32], // Destination contract address (32-byte)
}

impl SignMessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::new();

        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.amount.to_be_bytes());
        encoded.extend(self.l1_pubkey.as_bytes());
        encoded.extend(self.twine_address.as_bytes());
        encoded.extend(self.l1_token.as_bytes());
        encoded.extend(self.l2_token.as_bytes());

        encoded
    }
}

impl L1OriginTxPublicValues {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::new();
        encoded.extend(self.txn_type.as_bytes());
        encoded.extend_from_slice(&self.nonce.to_be_bytes());
        encoded.extend_from_slice(&self.chain_id.to_be_bytes());
        encoded.extend_from_slice(&self.slot_number.to_be_bytes());
        encoded.extend(self.l1_address.as_bytes());
        encoded.extend(self.l2_address.as_bytes());
        encoded.extend(self.l1_token_address.as_bytes());
        encoded.extend(self.l2_token_address.as_bytes());
        encoded.extend(self.amount.as_bytes());
        encoded.extend_from_slice(&self.message);
        encoded
    }
    pub fn calculate_deposit_hash(&self) -> [u8; 32] {
        let hash = Keccak256::digest(&self.abi_encode_packed());
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
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

