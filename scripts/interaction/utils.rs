use dirs;
use ethabi::{encode as abi_encode, Token};
use libsecp256k1::{sign, Message, SecretKey};
use sha3::{Digest, Keccak256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    program_pack::Pack,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{id as spl_token_program_id, instruction as token_instruction, state::Mint};
use tokens_gateway::{
    core::{state::SignMessageInfo},
};

pub fn get_rpc_url() -> Result<String, String> {
    if let Ok(url) = std::env::var("SOLANA_RPC_URL") {
        return Ok(url);
    }
    println!("SOLANA_RPC_URL not set. Defaulting to localnet at `http://127.0.0.1:8899`");
    Ok("http://127.0.0.1:8899".to_string())
}

pub fn get_rpc_client() -> RpcClient {
    let url = get_rpc_url().expect("RPC URL not set. Please set it before using the client.");
    RpcClient::new(url)
}

pub fn get_default_keypair() -> Keypair {
    let mut keypair_path = dirs::home_dir().expect("Could not get home directory");
    keypair_path.push(".config/solana/id.json");
    read_keypair_file(keypair_path).expect("Failed to read default keypair file")
}

pub fn get_or_create_ata(
    rpc: &RpcClient,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);
    if rpc.get_account(&ata).is_err() {
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                owner,
                mint,
                &spl_token_program_id(),
            );
        let tx = Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&payer.pubkey()),
            &[payer],
            rpc.get_latest_blockhash().unwrap(),
        );
        rpc.send_and_confirm_transaction(&tx).unwrap();
    }
    ata
}

pub fn create_spl_and_mint(
    rpc: &RpcClient,
    admin: &Keypair,
    mint_decimals: u8,
    mint_amount: u64,
) -> (Pubkey, Pubkey) {
    let mint = Keypair::new();
    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .expect("Failed to get rent exemption amount");
    let create_mint_acc_ix = system_instruction::create_account(
        &admin.pubkey(),
        &mint.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token_program_id(),
    );
    let init_mint_ix = token_instruction::initialize_mint(
        &spl_token_program_id(),
        &mint.pubkey(),
        &admin.pubkey(),
        None,
        mint_decimals,
    )
    .unwrap();
    let admin_ata = get_associated_token_address(&admin.pubkey(), &mint.pubkey());
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &admin.pubkey(),
        &admin.pubkey(),
        &mint.pubkey(),
        &spl_token_program_id(),
    );
    let mint_to_ix = token_instruction::mint_to(
        &spl_token_program_id(),
        &mint.pubkey(),
        &admin_ata,
        &admin.pubkey(),
        &[],
        mint_amount,
    )
    .unwrap();
    let blockhash = rpc.get_latest_blockhash().expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_acc_ix, init_mint_ix, create_ata_ix, mint_to_ix],
        Some(&admin.pubkey()),
        &[admin, &mint],
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx)
        .expect("Failed to send transaction");
    (mint.pubkey(), admin_ata)
}

pub fn abi_encode_sign_message(info: &SignMessageInfo) -> Vec<u8> {
    let tokens = vec![
        Token::Uint(info.nonce.into()),
        Token::Uint(info.chain_id.into()),
        Token::Uint(info.amount.into()),
        Token::String(info.l1_pubkey.clone()),
        Token::String(info.twine_address.clone()),
        Token::String(info.l1_token.clone()),
        Token::String(info.l2_token.clone()),
    ];
    abi_encode(&tokens)
}

pub fn get_asscoiated_token_account(
    wallet_address: &Pubkey,
    token_address: &Pubkey,
) -> Pubkey {
    let wallet_ata = get_associated_token_address(wallet_address, token_address);
    wallet_ata
}

pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hasher.finalize().into()
}

pub fn get_ethereum_signature(info: &SignMessageInfo, private_key_hex: &str) -> Vec<u8> {
    let abi_encoded = info.abi_encode_packed();
    let hash = keccak256(&abi_encoded);

    let secret_key_bytes =
        hex::decode(private_key_hex.trim_start_matches("0x")).expect("Invalid private key hex");
    let secret_key = SecretKey::parse_slice(&secret_key_bytes).expect("Invalid secret key");

    let message = Message::parse_slice(&hash).expect("Hash must be 32 bytes");
    let (signature, recovery_id) = sign(&message, &secret_key);
    let r = signature.r.b32();
    let s = signature.s.b32();
    let v = recovery_id.serialize();

    let mut sig_bytes = Vec::with_capacity(65);
    sig_bytes.extend_from_slice(&r);
    sig_bytes.extend_from_slice(&s);
    sig_bytes.push(v);

    sig_bytes
}
