use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
use solana_program::{program_pack::IsInitialized,program_error::ProgramError, pubkey::Pubkey};
use crate::core::error::ProgramCustomError;
/****************
 * Role Manager *
 ****************/

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TwineChainRoleManager {
    pub is_initialized: bool,
    pub chain_admin: Pubkey,
    pub twine_operator: Pubkey,
    pub token_gateway_program: Pubkey,
    pub roles: Vec<(Pubkey, RoleType)>,
}

/// Role types for authorization.
#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, PartialEq, Debug)]
pub enum RoleType {
    MessageAppender,
    TwineOperationHandler,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, PartialEq, Debug)]
pub enum TransactionType {
    Deposit,
    Withdraw,
    Message,
}
/*******************
 * Message Buffers *
 *******************/

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MessagesBuffer {
    pub is_initialized: bool,
    pub message_nonce: u64,
    pub chain_id: u64,
    pub messages: Vec<[u8; 32]>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MessagesReplicator {
    pub is_initialized: bool,
    pub start_nonce: u64,
    pub end_nonce: u64,
    pub messages: Vec<[u8; 32]>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct LayerZeroMessagesBuffer {
    pub is_initialized: bool,
    pub lz_nonce: u64,
    pub lz_messages: Vec<LayerZeroMessageInfo>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutionMessageBuffer {
    pub is_initialized: bool,
    pub withdrawals: Vec<ForcedWithdrawMessageInfo>,
}

/*****************
 * Data Storages *
 *****************/

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TwineChainStorage {
    pub is_initialized: bool,
    pub last_copied_message_start_nonce: u64,
    pub last_copied_message_end_nonce: u64,
    pub total_msg_handled_on_twine: u64,
    pub last_committed_batch_number: u64,
    pub last_finalized_batch_number: u64,
    pub groth16_vk: Vec<u8>,
    pub execution_vkey: String,
    pub inclusion_vkey: String,
    pub withdrawal_vkey: String,
    pub skip_verification: bool,
    pub last_committed_batch_hash: [u8; 32],
    pub last_finalized_batch_hash: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BatchPdaAccount {
    pub is_initialized: bool,
    pub batch_hash: [u8; 32],
}

/*******************************
 * Message Buffer Informations *
 *******************************/

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct DepositMessageInfo {
    pub txn_type: TransactionType,
    pub nonce: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub from_l1_pubkey: String,
    pub to_twine_address: String,
    pub l1_token: String,
    pub l2_token: String,
    pub amount: String,
    pub data: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ForcedWithdrawMessageInfo {
    pub txn_type: TransactionType,
    pub nonce: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub from_twine_address: String,
    pub to_l1_pubkey: String,
    pub l1_token: String,
    pub l2_token: String,
    pub amount: String,
    pub data: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct LayerZeroMessageInfo {
    pub message: String,
}

/*****************************
 * Data Storage Informations *
 *****************************/

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ChainCommitment {
    pub deposit_count: u64,
    pub deposit_rolling_hash: [u8; 32],
    pub withdraw_count: u64,
    pub withdraw_rolling_hash: [u8; 32],
    pub lz_transaction_count: u64,
    pub lz_transaction_rolling_hash: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CommitBatchInfo {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub transaction_root: [u8; 32],
    pub receipt_root: [u8; 32],
}

/********************************
 * Implementations for encoding *
 ********************************/
impl TransactionType {
    /// Return the variant as a single-byte array so we can
    /// `extend()` it into our Vec<u8>.
    pub fn as_bytes(self) -> [u8; 1] {
        [self as u8]
    }
    pub fn try_from(value: u8) -> Result<Self, ProgramError> {
        match value {
            v @ 0..=3 => Ok(unsafe { std::mem::transmute(v) }),
            _ => Err(ProgramCustomError::InvalidTransactionType.into()),
        }
    }
}

impl DepositMessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::with_capacity(DepositMessageInfo::LEN);
        encoded.extend(self.txn_type.as_bytes());
        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.slot_number.to_be_bytes());
        encoded.extend(self.from_l1_pubkey.as_bytes());
        encoded.extend(self.to_twine_address.as_bytes());
        encoded.extend(self.l1_token.as_bytes());
        encoded.extend(self.l2_token.as_bytes());
        encoded.extend(self.amount.as_bytes());
        encoded.extend(self.data.as_bytes());

        encoded
    }

    pub fn calculate_deposit_hash(&self) -> [u8; 32] {
        let hash = Keccak256::digest(&self.abi_encode_packed());
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
}

impl ForcedWithdrawMessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::with_capacity(ForcedWithdrawMessageInfo::LEN);

        encoded.extend(self.txn_type.as_bytes());
        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.slot_number.to_be_bytes());
        encoded.extend(self.from_twine_address.as_bytes());
        encoded.extend(self.to_l1_pubkey.as_bytes());
        encoded.extend(self.l1_token.as_bytes());
        encoded.extend(self.l2_token.as_bytes());
        encoded.extend(self.amount.as_bytes());
        encoded.extend(self.data.as_bytes());

        encoded
    }
    pub fn calculate_withdraw_hash(&self) -> [u8; 32] {
        let hash = Keccak256::digest(&self.abi_encode_packed());
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
}

impl LayerZeroMessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::new();
        encoded.extend(self.message.as_bytes());
        encoded
    }
}
/******************************************
 * Implementations for Length Calculation *
 *****************************************/

impl TwineChainStorage {
    pub const LEN: usize = 1    // is_initialized
        + 4 + 512   // groth16_vk
        + 4 + 66        // execution_vkey
        + 4 + 66    // inclusion_vkey
        + 4 + 66        // withdrawal_vkey
        + 1         // skip_verification  
        + 16        // last_finalized_batch
        + 16        // last_committed_batch
        + 16        // last_transcation_finalized_batch
        + 32; // last_finalized_receipt_root
}

impl DepositMessageInfo {
    pub const LEN: usize = 1
        + 8           // nonce (u64)
        + 8         // chain_id (u64)
        + 8         // slot_number(u64)
        + 4 + 44    // from_L1_publkey (String)
        + 4 + 42    // to_twine_address (String)
        + 4 + 44    // l1_token (String)
        + 4 + 42    // l2_token (String)
        + 4 + 32; // amount (String)
}

impl ForcedWithdrawMessageInfo {
    pub const LEN: usize = 1
        + 8       // nonce (u64)
        + 8         // chain_id (u64)
        + 8         // slot_number(u64)
        + 4 + 42    // from_twine_address (String)
        + 4 + 44    // to_l1_pubkey (String)
        + 4 + 44    // l1_token (String)
        + 4 + 42    // l2_token (String)
        + 4 + 32; // amount (String)
                  //Total: 250 bytes
}

impl BatchPdaAccount {
    pub const LEN: usize = 1 + 32;
}

/******************************************************
 * Implementations of IsInitialized function for PDAs *
 ******************************************************/

impl IsInitialized for TwineChainRoleManager {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl IsInitialized for MessagesBuffer {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for LayerZeroMessagesBuffer {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl IsInitialized for ExecutionMessageBuffer {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl IsInitialized for TwineChainStorage {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for BatchPdaAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
