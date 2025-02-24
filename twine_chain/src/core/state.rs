use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TwineChainRoleManager {
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

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TwineChainStorage {
    pub last_finalized_batch: BatchInfo,
    pub last_committed_batch: BatchInfo,
    pub last_finalized_receipt_root: [u8; 32],
    pub groth16_vk: Vec<u8>,
    pub execution_vkey: String,
    pub inclusion_vkey: String,
    pub withdrawal_vkey: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct BatchInfo {
    pub start_block: String,
    pub end_block: String,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BatchPdaAccount {
    pub infos: Vec<BlockInfo>,
    pub verified: bool,
    pub is_full: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct BlockInfo {
    pub previous_hash: [u8; 32],
    pub block_hash: [u8; 32],
    pub transaction_root: [u8; 32],
    pub receipt_root: [u8; 32],
}



#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositMessagesBuffer {
    pub deposit_nonce: u64,
    pub deposit_messages: Vec<DepositMessageInfo>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ForcedWithdrawMessagesBuffer {
    pub withdraw_nonce: u64,
    pub withdraw_messages: Vec<ForcedWithdrawMessageInfo>,
}

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


#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct LayerZeroMessagesBuffer {
    pub lz_nonce: u64,
    pub lz_messages: Vec<LayerZeroMessageInfo>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutionMessageBuffer {
    pub withdrawals: Vec<ForcedWithdrawMessageInfo>,
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

