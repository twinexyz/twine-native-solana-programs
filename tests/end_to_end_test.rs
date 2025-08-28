// #![allow(clippy::arithmetic_side_effects)]
// #[cfg(test)]
// mod helpers;

// use borsh::BorshDeserialize;
// use sha3::{Digest, Keccak256};
// use solana_program_test::*;
// use solana_sdk::{
//     signature::{Keypair, Signer},
//     transaction::Transaction,
// };

// use helpers::tokens_gateway_helper::{
//     fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
// };
// use tokens_gateway::{
//     core::{instruction as tokens_gateway_instruction, state::SignMessageInfo},
//     id as tokens_gateway_id,
//     utils::{
//         address_derivation::derive_native_token_vault_data, constants::ROLE_MANAGER_ACCOUNT_SIZE,
//     },
// };
// use twine_chain::{
//     core::{
//         instruction as twine_chain_instruction,
//         state::{
//             BlockInfo, ChainCommitment, CommitBatchInfo, DepositMessageInfo, DepositMessagesBuffer,
//             ForcedWithdrawMessageInfo, ForcedWithdrawMessagesBuffer, RoleType,
//         },
//     },
//     id as twine_chain_id,
//     utils::address_derivation::{
//         derive_deposit_message_buffer, derive_forced_withdraw_message_buffer,
//     },
// };

// use crate::helpers::tokens_gateway_helper::get_ethereum_signature;

// #[tokio::test]
// async fn end_to_end_test() {
//     let mut context = program_test().start_with_context().await;
//     let accounts = TokensGatewayAccounts::default();
//     let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();

//     fund_account_for_rent_exemption(
//         &mut context,
//         &payer,
//         &accounts.chain_admin.pubkey(),
//         ROLE_MANAGER_ACCOUNT_SIZE,
//         844073716442015,
//     )
//     .await;

//     // Required values for deposit and withdraw
//     let l1_token = "11111111111111111111111111111111".to_string();
//     let l2_token = "0x1234567890abcdef1234567890abcdef12345678".to_string();
//     let l1_decimals = 9u8;
//     let l2_decimals = 18u8;
//     let amount = 1u64;
//     let chain_admin = &accounts.chain_admin.pubkey();
//     let twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
//     let data = "".to_string();
//     let native_token_valut_data_account = derive_native_token_vault_data(&tokens_gateway_id()).0;
//     let sign_info = SignMessageInfo {
//         nonce: 1,
//         chain_id: 900,
//         amount: 1,
//         from_twine_address: twine_address.to_string(),
//         to_l1_pubkey: chain_admin.to_string(),
//         l1_token: l1_token.to_string(),
//         l2_token: l2_token.clone(),
//     };
//     let privkey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

//     let signature = get_ethereum_signature(&sign_info, privkey);
//     // Instructions to be executed
//     let mut initialize_and_bridge_instructions = vec![];
//     let mut finalization_instructions = vec![];

//     /********************************
//      * Twine Chain Initilizations *
//      *******************************/

//     // 1. Initialize twine chain role manager
//     initialize_and_bridge_instructions
//         .extend(twine_chain_instruction::initialize_twine_chain_role_manager(&chain_admin));

//     // 2. Initialize twine chain storage
//     initialize_and_bridge_instructions.extend(
//         twine_chain_instruction::initialize_twine_chain_storage(&chain_admin),
//     );

//     // 3. Initialize message buffers
//     initialize_and_bridge_instructions.extend(twine_chain_instruction::initialize_message_buffer(
//         &chain_admin,
//     ));

//     // 4. Give message appender role to native_token_vault_data account
//     initialize_and_bridge_instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
//         &chain_admin,
//         &native_token_valut_data_account,
//         RoleType::MessageAppender,
//     ));

//     // 5. Initialize Genesis Batch
//     initialize_and_bridge_instructions.extend(twine_chain_instruction::initialize_genesis_batch(
//         &accounts.chain_admin.pubkey(),
//         [0u8; 32],
//     ));

//     /*********************************
//      * Tokens Gateway Initilizations *
//      ********************************/

//     // 6. Initialize tokens gateway rolemanager
//     initialize_and_bridge_instructions
//         .extend(tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin));

//     // 7. Initialize tokens gateway
//     initialize_and_bridge_instructions.extend(
//         tokens_gateway_instruction::initialize_tokens_gateway(&chain_admin),
//     );

//     /***********************
//      * Bridge Instructions *
//      **********************/

//     // 8. Update gateway token mapping
//     initialize_and_bridge_instructions.extend(
//         tokens_gateway_instruction::update_gateway_token_mapping(
//             l1_token.clone(),
//             l2_token.clone(),
//             l1_decimals,
//             l2_decimals,
//             &chain_admin,
//         ),
//     );

//     // 9. Deposit Native token
//     initialize_and_bridge_instructions.extend(tokens_gateway_instruction::native_token_deposit(
//         &chain_admin,
//         twine_address.clone(),
//         l1_token.clone(),
//         l2_token.clone(),
//         2000000000,
//         data,
//     ));

//     let amount_to_withdraw = 1000000000u64;
//     // 10. Forced withdraw Native token
//     initialize_and_bridge_instructions.extend(
//         tokens_gateway_instruction::forced_native_token_withdrawal(
//             &chain_admin,
//             twine_address.clone(),
//             chain_admin.to_string(),
//             l1_token.clone(),
//             l2_token.clone(),
//             amount,
//             signature,
//         ),
//     );

//     // Making the transaction
//     let initialize_and_bridge_transaction = Transaction::new_signed_with_payer(
//         &initialize_and_bridge_instructions,
//         Some(&context.payer.pubkey()),
//         &[&context.payer, &accounts.chain_admin],
//         context.last_blockhash,
//     );

//     let transaction_result = context
//         .banks_client
//         .process_transaction(initialize_and_bridge_transaction)
//         .await;
//     println!(
//         "Initialization and Bridge Transaction status: {:?}",
//         transaction_result
//     );

//     // Get current deposit and withdraw message buffer account
//     let deposit_message_buffer_account = context
//         .banks_client
//         .get_account(derive_deposit_message_buffer(&twine_chain_id()).0)
//         .await
//         .unwrap()
//         .expect("Deposit Buffer account not found");

//     let forced_withdraw_message_buffer_account = context
//         .banks_client
//         .get_account(derive_forced_withdraw_message_buffer(&twine_chain_id()).0)
//         .await
//         .unwrap()
//         .expect("Forced withdraw buffer account not found");

//     let deposit_buffer_data =
//         DepositMessagesBuffer::deserialize(&mut &deposit_message_buffer_account.data[..])
//             .expect("Failed to deserialize deposit buffer");

//     let withdraw_buffer_data = ForcedWithdrawMessagesBuffer::deserialize(
//         &mut &forced_withdraw_message_buffer_account.data[..],
//     )
//     .expect("Failed to deserialize withdraw buffer");

//     /******************
//      * Commit a batch *
//      *****************/
//     let start_block = 1;
//     let end_block = 3;

//     // Blocks to commit
//     let first_block = CommitBatchInfo {
//         block_number: 1,
//         block_hash: [1u8; 32],
//         transaction_root: [0u8; 32],
//         receipt_root: [1u8; 32],
//     };

//     let second_block = CommitBatchInfo {
//         block_number: 2,
//         block_hash: [2u8; 32],
//         transaction_root: [0u8; 32],
//         receipt_root: [2u8; 32],
//     };

//     let third_block = CommitBatchInfo {
//         block_number: 3,
//         block_hash: [3u8; 32],
//         transaction_root: [0u8; 32],
//         receipt_root: [3u8; 32],
//     };

//     let commitment_batch_data = vec![first_block, second_block, third_block];

//     // 1. commit a batch
//     finalization_instructions.extend(twine_chain_instruction::commit_batch(
//         &accounts.chain_admin.pubkey(),
//         start_block,
//         end_block,
//         commitment_batch_data,
//     ));

//     /**********************
//      * Finalize the batch *
//      * 
//      *********************/
//     let block1_info = BlockInfo {
//         previous_hash: [0u8; 32],
//         block_hash: [1u8; 32],
//         transaction_root: [0u8; 32],
//         receipt_root: [1u8; 32],
//     };
//     let block2_info = BlockInfo {
//         previous_hash: [1u8; 32],
//         block_hash: [2u8; 32],
//         transaction_root: [0u8; 32],
//         receipt_root: [2u8; 32],
//     };
//     let block3_info = BlockInfo {
//         previous_hash: [2u8; 32],
//         block_hash: [3u8; 32],
//         transaction_root: [0u8; 32],
//         receipt_root: [3u8; 32],
//     };

//     // Calculate batch hash
//     let finalization_batch_data = vec![block1_info, block2_info, block3_info];

//     let mut calculated_batch_hash = [0u8; 32];
//     let mut serialized_batch_hash: Vec<u8> =
//         Vec::with_capacity(BlockInfo::LEN * finalization_batch_data.len());

//     for block in finalization_batch_data.clone() {
//         serialized_batch_hash.extend_from_slice(&block.abi_encode_packed());
//     }
//     let mut hasher = Keccak256::new();
//     hasher.update(serialized_batch_hash);

//     let encoded_batch_hash = hasher.finalize().to_vec();
//     calculated_batch_hash[..32].copy_from_slice(&encoded_batch_hash[..32]);

//     // Prepare public values
//     let mut public_values = Vec::with_capacity(48);
//     public_values.extend_from_slice(&start_block.to_be_bytes());
//     public_values.extend_from_slice(&end_block.to_be_bytes());
//     public_values.extend_from_slice(&calculated_batch_hash);

//     // 2. Finalize Batch
//     finalization_instructions.extend(twine_chain_instruction::finalize_batch(
//         &accounts.chain_admin.pubkey(),
//         start_block,
//         end_block,
//         public_values.clone(),
//         public_values,
//     ));

//     /****************************************
//      * Commit and Finalize the transactions *
//      ***************************************/

//     // Prepare inputs
//     let start_block: u64 = 1;
//     let end_block: u64 = 3;
//     let combined_receipt_root = calculate_combined_receipt_root(&finalization_batch_data);
//     let selected_deposits = [deposit_buffer_data.deposit_messages[0].clone()];
//     let selected_withdrawals = [withdraw_buffer_data.withdraw_messages[0].clone()];

//     let chain_commitment = ChainCommitment {
//         deposit_count: 1,
//         deposit_rolling_hash:[0u8; 32], //calculate_deposit_rolling_hash(&selected_deposits),
//         withdraw_count: 1,
//         withdraw_rolling_hash: calculate_withdraw_rolling_hash(&selected_withdrawals),
//         lz_transaction_count: 0,
//         lz_transaction_rolling_hash: [0u8; 32],
//     };

//     // Calculate transaction info
//     let mut encoded_chain_commitment = Vec::with_capacity(120);
//     encoded_chain_commitment.extend_from_slice(&chain_commitment.deposit_count.to_be_bytes());
//     encoded_chain_commitment.extend_from_slice(&chain_commitment.deposit_rolling_hash);
//     encoded_chain_commitment.extend_from_slice(&chain_commitment.withdraw_count.to_be_bytes());
//     encoded_chain_commitment.extend_from_slice(&chain_commitment.withdraw_rolling_hash);
//     encoded_chain_commitment
//         .extend_from_slice(&chain_commitment.lz_transaction_count.to_be_bytes());
//     encoded_chain_commitment.extend_from_slice(&chain_commitment.lz_transaction_rolling_hash);

//     let mut transaction_info = vec![0u8; 288];
//     transaction_info[0..8].copy_from_slice(&start_block.to_be_bytes());
//     transaction_info[8..16].copy_from_slice(&end_block.to_be_bytes());
//     transaction_info[16..48].copy_from_slice(&combined_receipt_root);
//     transaction_info[168..288].copy_from_slice(&encoded_chain_commitment);

//     // 3. Commit and Finalize Transaction
//     finalization_instructions.extend(twine_chain_instruction::commit_and_finalize_transaction(
//         &accounts.chain_admin.pubkey(),
//         start_block,
//         end_block,
//         transaction_info.clone(),
//         transaction_info,
//     ));

//     /*****************************
//      * Finalize Naive Withdrawal *
//      ****************************/
//     let chain_id = 9u64;
//     let block_number = 1u64;
//     let nonce = 1u64;
//     let is_forced_withdrawal = 1;
//     let receipt_root: [u8; 32] = [
//         0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
//         0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
//         0x1a, 0x1b,
//     ];
//     let inclusion_proof = vec![];
//     let l1_receiver_address = accounts.chain_admin.pubkey();
//     finalization_instructions.extend(
//         tokens_gateway_instruction::finalize_native_token_withdrawal(
//             chain_id,
//             block_number,
//             nonce,
//             is_forced_withdrawal,
//             receipt_root,
//             l1_receiver_address,
//             l1_token.clone(),
//             l2_token.clone(),
//             amount_to_withdraw.to_string(),
//             inclusion_proof,
//         ),
//     );
//     // Making the transaction:
//     let finalization_transaction = Transaction::new_signed_with_payer(
//         &finalization_instructions,
//         Some(&context.payer.pubkey()),
//         &[&context.payer, &accounts.chain_admin],
//         context.last_blockhash,
//     );
//     let error = context
//         .banks_client
//         .process_transaction(finalization_transaction)
//         .await;
//     println!("Finalization Transaction Status: {:?}", error);
// }

// fn calculate_combined_receipt_root(batch_info: &Vec<BlockInfo>) -> [u8; 32] {
//     let mut calculated_receipt_root = [0u8; 32];

//     let mut receipt_root_vector = Vec::new();
//     for blocks in batch_info {
//         receipt_root_vector.extend_from_slice(&blocks.receipt_root);
//     }
//     let mut hasher = Keccak256::new();
//     hasher.update(receipt_root_vector);
//     let receipt_root = hasher.finalize().to_vec();

//     calculated_receipt_root[..32].copy_from_slice(&receipt_root[..32]);

//     calculated_receipt_root
// }

// fn calculate_deposit_rolling_hash(selected_deposits: &[DepositMessageInfo]) -> [u8; 32] {
//     let mut deposit_rolling_hash = [0u8; 32];
//     let mut serialized_deposit_data = Vec::new();

//     for deposits in selected_deposits {
//         serialized_deposit_data.extend_from_slice(&deposits.abi_encode_packed());
//     }

//     let mut hasher = Keccak256::new();
//     hasher.update(serialized_deposit_data);
//     let combined_deposit_hash = hasher.finalize().to_vec();

//     deposit_rolling_hash[..32].copy_from_slice(&combined_deposit_hash[..32]);

//     return deposit_rolling_hash;
// }

// fn calculate_withdraw_rolling_hash(selected_withdrawals: &[ForcedWithdrawMessageInfo]) -> [u8; 32] {
//     let mut withdraw_rolling_hash = [0u8; 32];
//     let mut serialized_withdraw_data = Vec::new();

//     for withdrawals in selected_withdrawals {
//         serialized_withdraw_data.extend_from_slice(&withdrawals.abi_encode_packed());
//     }

//     let mut hasher = Keccak256::new();
//     hasher.update(serialized_withdraw_data);
//     let combined_withdraw_hash = hasher.finalize().to_vec();

//     withdraw_rolling_hash[..32].copy_from_slice(&combined_withdraw_hash[..32]);

//     return withdraw_rolling_hash;
// }
