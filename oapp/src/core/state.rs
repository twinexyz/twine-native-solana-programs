use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

// PDA seeds
pub const PEER_SEED: &[u8] = b"Peer";
pub const STORE_SEED: &[u8] = b"Store";
pub const LZ_RECEIVE_TYPES_SEED: &[u8] = b"LzReceiveTypes";
pub const LZ_COMPOSE_TYPES_SEED: &[u8] = b"LzComposeTypes";

pub const OAPP_SEED: &[u8] = b"OApp";
pub const NONCE_SEED: &[u8] = b"Nonce";
pub const ENDPOINT_SEED: &[u8] = b"Endpoint";
pub const MESSAGE_LIB_SEED: &[u8] = b"MessageLib";
pub const SEND_CONFIG_SEED: &[u8] = b"SendConfig";
pub const EVENT_SEED: &[u8] = b"__event_authority";
pub const PENDING_NONCE_SEED: &[u8] = b"PendingNonce";
pub const RECEIVE_CONFIG_SEED: &[u8] = b"ReceiveConfig";
pub const SEND_LIBRARY_CONFIG_SEED: &[u8] = b"SendLibraryConfig";
pub const RECEIVE_LIBRARY_CONFIG_SEED: &[u8] = b"ReceiveLibraryConfig";

pub const EXECUTOR_CONFIG_SEED: &[u8] = b"ExecutorConfig";
pub const DVN_CONFIG_SEED: &[u8] = b"DvnConfig";


// Anchor instruction use an 8-byte discriminator followed by the serialized parameters.
// Example:  Discriminator for "register_oapp" (sha256("global:register_oapp")[..8]):
pub const ENDPOINT_SEND_DISCRIMINATOR: [u8; 8] = [0x66, 0xfb, 0x14, 0xbb, 0x41, 0x4b, 0x0c, 0x45];
pub const ENDPOINT_SET_CONFIG_DISCEIMINATOR: [u8; 8] =
    [0x6c, 0x9e, 0x9a, 0xaf, 0xd4, 0x62, 0x34, 0x42];
pub const ENDPOINT_INIT_NONCE_DISCRIMINATOR: [u8; 8] =
    [0xcc, 0xab, 0x10, 0xd6, 0xb6, 0xbf, 0x1b, 0xc4];
pub const ENDPOINT_INIT_CONFIG_DISCRIMINATOR: [u8; 8] =
    [0x17, 0xeb, 0x73, 0xe8, 0xa8, 0x60, 0x01, 0xe7];
pub const ENDPOINT_SET_SEND_LIB_DISCRIMINATOR: [u8; 8] =
    [0xfb, 0x76, 0x4e, 0x9e, 0x86, 0x95, 0x81, 0x05];
pub const ENDPOINT_REGISTER_OAPP_DISCRIMINATOR: [u8; 8] =
    [0x81, 0x59, 0x47, 0x44, 0x0B, 0x52, 0xD2, 0x7D];
pub const ENDPOINT_INIT_SEND_LIBRARY_DISCRIMINATOR: [u8; 8] =
    [0x9c, 0x18, 0xeb, 0x78, 0x49, 0xc1, 0x90, 0x13];
pub const ENDPOINT_INIT_RECEIVE_LIBRARY_DISCRIMINATOR: [u8; 8] =
    [0xc5, 0x72, 0x51, 0x64, 0x2d, 0xe9, 0x24, 0xe6];
pub const ENDPOINT_SET_RECEIVE_LIBRARY_DISCRIMINATOR: [u8; 8] =
    [0xdf, 0xac, 0xb4, 0x69, 0xa5, 0xa1, 0x93, 0xe4];

// OApp Store account data structure
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct Store {
    pub admin: Pubkey,
    pub endpoint_program: Pubkey,
    pub bump: u8,
    pub last_msg_size: u32,
    pub last_msg: [u8; 256],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct InitSendLibraryParams {
    pub sender: Pubkey,
    pub eid: u32,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct InitReceiveLibraryParams {
    pub receiver: Pubkey,
    pub eid: u32,
}

// Peer configuration account
#[derive(BorshSerialize, BorshDeserialize)]
pub struct PeerConfig {
    pub peer_address: [u8; 32],
    pub bump: u8,
}

// Instruction parameter structures
#[derive(BorshSerialize, BorshDeserialize)]
pub struct InitStoreParams {
    pub endpoint_id: Pubkey,
}

// Instruction parameter structures
#[derive(BorshSerialize, BorshDeserialize)]
pub struct RegisterOAppParams {
    pub delegate: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct InitNonceParams {
    pub local_oapp: Pubkey, // the PDA of the OApp
    pub remote_eid: u32,
    pub remote_oapp: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SetPeerParams {
    pub remote_chain: u32,        // LayerZero chain ID of the remote chain
    pub remote_address: [u8; 32], // Trusted remote contract address (padded/encoded to 32 bytes)
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct InitConfigParams {
    pub oapp: Pubkey,
    pub eid: u32,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SendMsgParams {
    pub dst_eid: u32,       // Destination chain ID
    pub receiver: [u8; 32], // Destination contract address (32-byte)
    pub message: Vec<u8>,   // Arbitrary payload to send
    pub options: Vec<u8>,
    pub native_fee: u64,    // Fee in native tokens (e.g. lamports) to pay for messaging
    pub lz_token_fee: u64,    // Fee in ZRO token (if using LayerZero token for fees)
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct LzAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SetSendLibraryParams {
    pub sender: Pubkey,
    pub eid: u32,
    pub new_lib: Pubkey,
}
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SetConfigParams {
    pub oapp: Pubkey,
    pub eid: u32,
    pub config_type: u32,
    pub config: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct UlnConfig {
    pub confirmations: u64,
    pub required_dvn_count: u8,
    pub optional_dvn_count: u8,
    pub optional_dvn_threshold: u8,
    pub required_dvns: Vec<Pubkey>,
    pub optional_dvns: Vec<Pubkey>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutorConfig {
    pub max_message_size: u32,
    pub executor: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct RegisterLibraryParams {
    pub lib_program: Pubkey,
    pub lib_type: MessageLibType,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum MessageLibType {
    Send,
    Receive,
    SendAndReceive,
}
impl Store {
    pub const LEN: usize = 32 + 32 + 1 + 4 + 256;
}
