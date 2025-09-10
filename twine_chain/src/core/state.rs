use crate::core::error::ProgramCustomError;
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
use solana_program::{program_error::ProgramError, program_pack::IsInitialized, pubkey::Pubkey};
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
    pub messages_rolling_hash: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct DetailedMessagesBuffer {
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
pub struct MessageInfo {
    pub txn_type: TransactionType,
    pub nonce: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub l1_pubkey: String,
    pub twine_address: String,
    pub l1_token: String,
    pub l2_token: String,
    pub amount: String,
    pub data: Vec<u8>,
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

/**********
 * Events *
 *********/
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MessageTransactionEvent {
    pub event: String,
    pub nonce: u64,
    pub slot_number: u64,
    pub l1_pubkey: String,
    pub twine_address: String,
    pub l1_token: String,
    pub l2_token: String,
    pub chain_id: u64,
    pub amount: String,
    pub data: Vec<u8>,
    pub message_type: String,
    pub previous_rolling_hash: [u8;32],
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CommitedBatchEvent {
    pub event: String,
    pub batch_number: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub batch_hash: [u8; 32],
   
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FinalizedBatchEvent {
    pub event: String,
    pub batch_number: u64,
    pub messages_handled_on_twine: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub batch_hash: [u8; 32],
    
}

/********************************
 * Implementations methods *
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

impl MessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::with_capacity(MessageInfo::LEN);
        encoded.extend(self.txn_type.as_bytes());
        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.slot_number.to_be_bytes());
        let mut hasher = Keccak256::new();
        hasher.update(self.data.clone());
        let data_hash = hasher.finalize();
        encoded.extend(data_hash.as_slice());
        encoded.extend(self.l1_pubkey.to_lowercase().as_bytes());
        encoded.extend(self.twine_address.to_lowercase().as_bytes());
        encoded.extend(self.l1_token.to_lowercase().as_bytes());
        encoded.extend(self.l2_token.to_lowercase().as_bytes());
        encoded.extend(self.amount.as_bytes());

        encoded
    }

    pub fn calculate_message_hash(&self) -> [u8; 32] {
        let hash = Keccak256::digest(&self.abi_encode_packed());
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }

}
impl MessagesBuffer {
      pub fn update_rolling_hash(&mut self, new_message_hash: &[u8; 32]) {
        let mut hasher = Keccak256::new();
        hasher.update(&self.messages_rolling_hash);
        hasher.update(new_message_hash);
        let result = hasher.finalize();
        
        self.messages_rolling_hash.copy_from_slice(&result);
        self.message_nonce += 1;
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

impl MessageInfo {
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

impl IsInitialized for DetailedMessagesBuffer {
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
