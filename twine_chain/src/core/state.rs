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
    pub last_finalized_batch: BatchInfo,
    pub last_committed_batch: BatchInfo,
    pub last_finalized_receipt_root: [u8; 32],
    pub groth16_vk: Vec<u8>,
    pub execution_vkey: String,
    pub inclusion_vkey: String,
    pub withdrawal_vkey: String,
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

/******************************************
 * Implementations for Length Calculation *
 ******************************************/

impl DepositMessageInfo {
    pub const LEN: usize = 8       // nonce (u64)
        + 32    // to_twine_address (String)
        + 32    // l1_token (String)
        + 32    // l2_token (String)
        + 8     // chain_id (u64)
        + 32    // amount (String)
        + 8; // slot_number(u64)
}

impl ForcedWithdrawMessageInfo {
    pub const LEN: usize = 8       // nonce (u64)
        + 32    // from_twine_address (String)
        + 32    // to_l1_pubkey (String)
        + 32    // l1_token (String)
        + 32    // l2_token (String)
        + 8     // chain_id (u64)
        + 32    // amount (String)
        + 8; // slot_number(u64)
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
