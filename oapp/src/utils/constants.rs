// PDA seeds
pub const PEER_SEED: &[u8] = b"Peer";
pub const STORE_SEED: &[u8] = b"Store";
pub const LZ_RECEIVE_TYPES_SEED: &[u8] = b"LzReceiveTypes";
pub const LZ_COMPOSE_TYPES_SEED: &[u8] = b"LzComposeTypes";

pub const ULN_SEED: &[u8] = b"MessageLib";
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

// Constants
pub const ENDPOINT_ID: &str = "76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6";
pub const SEND_LIBRARY_PROGRAM_ID: &str = "7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH";
pub const SEND_LIBRARY_INFO_ID: &str = "2XgGZG4oP29U3w5h4nTk1V2LFHL23zKDPJjs3psGzLKQ";

// Anchor instruction use an 8-byte discriminator followed by the serialized parameters.
// Example:  Discriminator for "register_oapp" (sha256("global:register_oapp")[..8]):
pub const ENDPOINT_SEND_DISCRIMINATOR: [u8; 8] = 
    [0x66, 0xfb, 0x14, 0xbb, 0x41, 0x4b, 0x0c, 0x45];
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
