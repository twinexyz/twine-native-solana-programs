use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};
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
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub enum RoleType {
    MessageAppender,
    TwineOperationHandler,
}

/*******************
 * Message Buffers *
 *******************/

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositMessagesBuffer {
    pub is_initialized: bool,
    pub deposit_nonce: u64,
    pub deposit_messages: Vec<DepositMessageInfo>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ForcedWithdrawMessagesBuffer {
    pub is_initialized: bool,
    pub withdraw_nonce: u64,
    pub withdraw_messages: Vec<ForcedWithdrawMessageInfo>,
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
    pub groth16_vk: Vec<u8>,
    pub execution_vkey: String,
    pub inclusion_vkey: String,
    pub withdrawal_vkey: String,
    pub last_finalized_batch: BatchInfo,
    pub last_committed_batch: BatchInfo,
    pub last_transcation_finalized_batch: BatchInfo,
    pub last_finalized_receipt_root: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BatchPdaAccount {
    pub is_initialized: bool,
    pub infos: Vec<BlockInfo>,
    pub verified: bool,
    pub is_full: bool,
}

/*******************************
 * Message Buffer Informations *
 *******************************/

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct DepositMessageInfo {
    pub nonce: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub from_l1_pubkey: String,
    pub to_twine_address: String,
    pub l1_token: String,
    pub l2_token: String,
    pub amount: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ForcedWithdrawMessageInfo {
    pub nonce: u64,
    pub chain_id: u64,
    pub slot_number: u64,
    pub from_twine_address: String,
    pub to_l1_pubkey: String,
    pub l1_token: String,
    pub l2_token: String,
    pub amount: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct LayerZeroMessageInfo {
    pub message: String,
}

/*****************************
 * Data Storage Informations *
 *****************************/

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct BatchInfo {
    pub start_block: u64,
    pub end_block: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct BlockInfo {
    pub previous_hash: [u8; 32],
    pub block_hash: [u8; 32],
    pub transaction_root: [u8; 32],
    pub receipt_root: [u8; 32],
}

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
impl BlockInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::with_capacity(BlockInfo::LEN);

        encoded.extend(self.previous_hash);
        encoded.extend(self.block_hash);
        encoded.extend(self.transaction_root);
        encoded.extend(self.receipt_root);

        encoded
    }
}

impl DepositMessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::with_capacity(DepositMessageInfo::LEN);
        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.slot_number.to_be_bytes());
        encoded.extend(self.from_l1_pubkey.as_bytes());
        encoded.extend(self.to_twine_address.as_bytes());
        encoded.extend(self.l1_token.as_bytes());
        encoded.extend(self.l2_token.as_bytes());
        encoded.extend(self.amount.as_bytes());

        encoded
    }
}

impl ForcedWithdrawMessageInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::with_capacity(ForcedWithdrawMessageInfo::LEN);

        encoded.extend(self.nonce.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.slot_number.to_be_bytes());
        encoded.extend(self.from_twine_address.as_bytes());
        encoded.extend(self.to_l1_pubkey.as_bytes());
        encoded.extend(self.l1_token.as_bytes());
        encoded.extend(self.l2_token.as_bytes());
        encoded.extend(self.amount.as_bytes());

        encoded
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
        + 4 + 128   // execution_vkey
        + 4 + 128   // inclusion_vkey
        + 4 + 128   // withdrawal_vkey
        + 16        // last_finalized_batch
        + 16        // last_committed_batch
        + 16        // last_transcation_finalized_batch
        + 32;       // last_finalized_receipt_root
}

impl DepositMessageInfo {
    pub const LEN: usize = 8           // nonce (u64)
        + 8         // chain_id (u64)
        + 8         // slot_number(u64)
        + 4 + 44    // from_L1_publkey (String)
        + 4 + 42    // to_twine_address (String)
        + 4 + 44    // l1_token (String)
        + 4 + 42    // l2_token (String)
        + 4 + 32; // amount (String)
                  // Total: 250 bytes
}

impl ForcedWithdrawMessageInfo {
    pub const LEN: usize = 8       // nonce (u64)
        + 8         // chain_id (u64)
        + 8         // slot_number(u64)
        + 4 + 42    // from_twine_address (String)
        + 4 + 44    // to_l1_pubkey (String)
        + 4 + 44    // l1_token (String)
        + 4 + 42    // l2_token (String)
        + 4 + 32; // amount (String)
                  //Total: 250 bytes
}

impl BlockInfo {
    pub const LEN: usize = 32   //prev_hash(32)
    + 32    //block_hash(32)    
    + 32    //transaction_root(32)
    + 32; //receipt_root(32)
}

/******************************************************
 * Implementations of IsInitialized function for PDAs *
 ******************************************************/

impl IsInitialized for TwineChainRoleManager {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl IsInitialized for DepositMessagesBuffer {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl IsInitialized for ForcedWithdrawMessagesBuffer {
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
