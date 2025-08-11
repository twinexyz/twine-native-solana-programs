#[cfg(test)]
mod helpers;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::tokens_gateway_helper::{
    create_spl_and_mint, fund_account_for_rent_exemption, get_or_create_ata, program_test,
    TokensGatewayAccounts,
};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction,
    id as tokens_gateway_id,
    utils::{
        address_derivation::{derive_spl_tokens_vault_data, derive_spl_vault_authority},
        constants::ROLE_MANAGER_ACCOUNT_SIZE,
    },
};
use twine_chain::core::{instruction as twine_chain_instruction, state::RoleType};

#[tokio::test]
async fn l2_spl_withdrawal_finalized_succeed() {
    let mut context = program_test().start_with_context().await;
    let accounts = TokensGatewayAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        ROLE_MANAGER_ACCOUNT_SIZE,
        844073716442015,
    )
    .await;

    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 8000000000000000u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let batch_number = 1u64;
    let nonce = 12u64;
    let batch_hash = [1u8; 32];
    let l2_token = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let execution_proof = vec![];
    let mut public_values = vec![];
    let (spl_token_pubkey, user_token_account) =
        create_spl_and_mint(&mut context, &accounts.chain_admin, 9, 9000000000).await;
    let spl_token_vault = get_or_create_ata(
        &mut context,
        &accounts.chain_admin,
        &derive_spl_vault_authority(&tokens_gateway_id()).0,
        &spl_token_pubkey,
    )
    .await;
    let l1_token = spl_token_pubkey.to_string();
    let data = "".to_string();
    let spl_token_valut_data_account = derive_spl_tokens_vault_data(&tokens_gateway_id()).0;
    public_values.extend_from_slice(&batch_number.to_be_bytes());
    public_values.extend_from_slice(&nonce.to_be_bytes());
    public_values.extend_from_slice(&batch_hash);
    public_values.extend_from_slice(&user_token_account.to_string().as_bytes());
    public_values.extend_from_slice(&l1_token.as_bytes());
    public_values.extend_from_slice(&l2_token.as_bytes());
    public_values.extend_from_slice(&amount.to_string().as_bytes());
    println!("The public values{:?}", public_values);
    println!("The hex values {:?}", hex::encode(&public_values));

    let mut instructions = vec![];
    instructions.extend(twine_chain_instruction::initialize_twine_chain_role_manager(&chain_admin));
    instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &chain_admin,
    ));
    instructions.extend(twine_chain_instruction::initialize_message_buffer(
        &chain_admin,
    ));
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
        &spl_token_valut_data_account,
        RoleType::MessageAppender,
    ));
    instructions
        .extend(tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin));
    instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &chain_admin,
    ));
    instructions.extend(tokens_gateway_instruction::update_gateway_token_mapping(
        l1_token.clone(),
        l2_token.clone(),
        l1_decimals,
        l2_decimals,
        &chain_admin,
    ));
    instructions.extend(tokens_gateway_instruction::spl_token_deposit(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        &spl_token_vault,
        receiver_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        4000000000,
        data,
    ));
    instructions.extend(tokens_gateway_instruction::execute_l2_spl_withdrawal(
        &spl_token_pubkey,
        &spl_token_vault,
        user_token_account,
        public_values,
        execution_proof,
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
}
