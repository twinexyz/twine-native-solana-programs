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
pub const SEND_LIBRARY_CONFIG_SEED: &[u8] = b"SendLibraryConfig";

// Endpoint instruction discriminators
pub const ENDPOINT_SEND_DISCRIMINATOR: [u8; 8] = [0x66, 0xfb, 0x14, 0xbb, 0x41, 0x4b, 0x0c, 0x45];
pub const ENDPOINT_SET_CONFIG_DISCEIMINATOR: [u8; 8] =
    [0x6c, 0x9e, 0x9a, 0xaf, 0xd4, 0x62, 0x34, 0x42];
pub const ENDPOINT_SET_SEND_LIB_DISCRIMINATOR: [u8; 8] =
    [0x97, 0xf8, 0x5f, 0x9e, 0xfc, 0x28, 0x97, 0xf5];
pub const ENDPOINT_REGISTER_OAPP_DISCRIMINATOR: [u8; 8] =
    [0x81, 0x59, 0x47, 0x44, 0x0B, 0x52, 0xD2, 0x7D];

// OApp Store account data structure
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct Store {
    pub admin: Pubkey,
    pub endpoint_program: Pubkey,
    pub bump: u8,
    pub last_msg_size: u32,
    pub last_msg: [u8; 256],
}

// Peer configuration account
#[derive(BorshSerialize, BorshDeserialize)]
pub struct PeerConfig {
    pub peer_address: [u8; 32],
    pub bump: u8,
}

// lz_receive_types and lz_compose_types account data (just stores a reference to Store)
#[derive(BorshSerialize, BorshDeserialize)]
pub struct TypesAccount {
    pub store: Pubkey, // The OApp Store address
}

// Instruction parameter structures
#[derive(BorshSerialize, BorshDeserialize)]
pub struct InitStoreParams {
    pub endpoint_id: Pubkey, // LayerZero Endpoint program ID to use
                             // admin can be taken from signer
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SetPeerParams {
    pub remote_chain: u32,        // LayerZero chain ID of the remote chain
    pub remote_address: [u8; 32], // Trusted remote contract address (padded/encoded to 32 bytes)
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SendMsgParams {
    pub dst_eid: u32,       // Destination chain ID
    pub dst_oapp: [u8; 32], // Destination contract address (32-byte)
    pub message: Vec<u8>,   // Arbitrary payload to send
    pub options: Vec<u8>,
    pub native_fee: u64, // Fee in native tokens (e.g. lamports) to pay for messaging
    pub zro_fee: u64,    // Fee in ZRO token (if using LayerZero token for fees)
                         // (For simplicity, we use no AdapterParameters "options"; we pass an empty options vector)
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct LzReceiveParams {
    pub src_chain: u32,   // Source chain ID
    pub sender: [u8; 32], // Sender address from the source chain (32-byte representation)
    pub nonce: u64,       // Message nonce
    pub guid: [u8; 32],   // Unique message GUID
    pub message: Vec<u8>, // Payload bytes
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
struct ExecutorConfig {
    max_message_size: u32,
    executor: Pubkey,
}
