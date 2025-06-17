#![allow(dead_code)]
use {
    dirs,
    ethabi::{encode as abi_encode, Token},
    libsecp256k1::{sign, Message, SecretKey},
    sha3::{Digest, Keccak256},
    solana_program::{pubkey::Pubkey, system_program},
    solana_program_test::{processor, ProgramTest, ProgramTestContext},
    solana_sdk::{
        program_pack::Pack,
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    spl_associated_token_account::get_associated_token_address,
    spl_token::{id as spl_token_program_id, instruction as token_instruction, state::Mint},
    tokens_gateway::{
        core::processor as tokens_gateway_processor, core::state::SignMessageInfo,
        utils::address_derivation::derive_gateway_role_manager, ID as tokens_gateway_ID,
    },
    twine_chain::core::processor as twine_chain_processor,
    twine_chain::ID as twine_chain_ID,
};

pub fn program_test() -> ProgramTest {
    let mut program_test = ProgramTest::default();

    program_test.add_program(
        "tokens_gateway",
        tokens_gateway_ID,
        processor!(tokens_gateway_processor::process_instruction),
    );

    program_test.add_program(
        "twine_chain",
        twine_chain_ID,
        processor!(twine_chain_processor::process_instruction),
    );

    program_test.prefer_bpf(false);
    program_test
}

pub async fn get_or_create_ata(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);
    let maybe_account = context.banks_client.get_account(ata).await.unwrap();

    if maybe_account.is_none() {
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
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();
    }

    ata
}

pub async fn create_spl_and_mint(
    context: &mut ProgramTestContext,
    admin: &Keypair,
    mint_decimals: u8,
    mint_amount: u64,
) -> (Pubkey, Pubkey) {
    let mint = Keypair::new();
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);

    let create_mint_acc_ix = system_instruction::create_account(
        &admin.pubkey(),
        &mint.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );

    let init_mint_ix = token_instruction::initialize_mint(
        &spl_token::id(),
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
        &spl_token::id(),
        &mint.pubkey(),
        &admin_ata,
        &admin.pubkey(),
        &[],
        mint_amount,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_mint_acc_ix, init_mint_ix, create_ata_ix, mint_to_ix],
        Some(&admin.pubkey()),
        &[admin, &mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    (mint.pubkey(), admin_ata)
}

#[derive(Debug, PartialEq)]
pub struct TokensGatewayAccounts {
    pub role_manager: Pubkey,
    pub chain_admin: Keypair,
    pub system_program: Pubkey,
}

fn get_default_keypair() -> Keypair {
    let mut keypair_path = dirs::home_dir().expect("Could not get home directory");
    keypair_path.push(".config/solana/id.json");
    read_keypair_file(keypair_path).expect("Failed to read default keypair file")
}

pub async fn fund_account_for_rent_exemption(
    context: &mut ProgramTestContext,
    payer: &dyn Signer,
    recipient: &Pubkey,
    space: usize,
    buffer_lamports: u64,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let min_balance = rent.minimum_balance(space);
    let fund_amount = min_balance + buffer_lamports;

    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            recipient,
            fund_amount,
        )],
        Some(&payer.pubkey()),
        &[payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
}

impl Default for TokensGatewayAccounts {
    fn default() -> Self {
        let chain_admin = get_default_keypair();
        Self {
            role_manager: derive_gateway_role_manager().0,
            chain_admin,
            system_program: system_program::id(),
        }
    }
}

pub fn abi_encode_sign_message(info: &SignMessageInfo) -> Vec<u8> {
    let tokens = vec![
        Token::Uint(info.nonce.into()),
        Token::Uint(info.chain_id.into()),
        Token::Uint(info.amount.into()),
        Token::String(info.from_twine_address.clone()),
        Token::String(info.to_l1_pubkey.clone()),
        Token::String(info.l1_token.clone()),
        Token::String(info.l2_token.clone()),
    ];
    abi_encode(&tokens)
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
    let v = recovery_id.serialize(); // 0 or 1

    let mut sig_bytes = Vec::with_capacity(65);
    sig_bytes.extend_from_slice(&r);
    sig_bytes.extend_from_slice(&s);
    sig_bytes.push(v);

    sig_bytes
}
