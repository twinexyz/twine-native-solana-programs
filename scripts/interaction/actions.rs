use crate::action_commands::Commands;
use crate::operations::{
    create_spl_token::create_spl_token,
    deposit_native_token::native_token_deposit,
    deposit_spl_token::spl_token_deposit,
    execute_native_l2_withdrawal::execute_native_l2_withdrawal,
    execute_spl_l2_withdrawal::execute_spl_l2_withdrawal,
    forced_native_withdrawal::forced_native_withdrawal,
    forced_spl_withdrawal::forced_spl_withdrawal,
    get_all_pdas::{get_all_pdas, get_batch_pda},
    get_associated_token_account::get_associated_token_account,
    get_pdas_data::{
        get_messages_buffer_data, get_payouts_buffer_data, get_twine_chain_storage_data,
    },
    initialize_programs::initialize_twine_solana_programs,
    process_native_l1_forced_withdrawal::process_native_l1_forced_withdrawal,
    process_spl_l1_forced_withdrawal::process_spl_l1_forced_withdrawal,
    process_native_l1_refund::process_native_l1_refund,
    process_spl_l1_refund::process_spl_l1_refund,
    token_mapping::update_token_mapping,
    role_operations_twine_chain::add_role_in_twine_chain,
    role_operations_tokens_gateway::add_role_in_tokens_gateway,
};

pub fn handle_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::InitializePrograms {} => {
            let result = initialize_twine_solana_programs();
            println!("Result: {:?}", result);
        }
        Commands::CreateSplToken {} => {
            let result = create_spl_token();
            println!("Result: {:?}", result);
        }
        Commands::AddRoleInTwineChain {role_type,user_pubkey} => {
            let result = add_role_in_twine_chain(role_type,user_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::AddRoleInTokensGateway {role_type,user_pubkey} => {
            let result = add_role_in_tokens_gateway(role_type,user_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::GetAssociatedTokenAccount {
            wallet_address,
            spl_token_pubkey,
        } => {
            let result = get_associated_token_account(wallet_address, spl_token_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::GetAllPdas {} => {
            let result = get_all_pdas();
            println!("Result: {:?}", result);
        }
        Commands::GetBatchPda { batch_number } => {
            let result = get_batch_pda(batch_number);
            println!("Result: {:?}", result);
        }

        Commands::GetMessagesBufferData {} => {
            let result = get_messages_buffer_data();
            println!("Result: {:?}", result);
        }
        Commands::GetExecutedPayoutsBufferData {} => {
            let result = get_payouts_buffer_data();
            println!("Result: {:?}", result);
        }
        Commands::GetTwineChainStorageData {} => {
            let result = get_twine_chain_storage_data();
            println!("Result: {:?}", result);
        }
        Commands::TokenMapping {
            l1_token,
            l2_token,
            l1_decimals,
            l2_decimals,
        } => {
            let result = update_token_mapping(l1_token, l2_token, l1_decimals, l2_decimals);
            println!("Result: {:?}", result);
        }

        Commands::DepositNativeToken {
            l1_token,
            l2_token,
            receiver_twine_address,
            amount,
            data,
        } => {
            let result =
                native_token_deposit(l1_token, l2_token, receiver_twine_address, amount, data);
            println!("Result: {:?}", result);
        }
        Commands::DepositSplToken {
            l1_token,
            l2_token,
            receiver_twine_address,
            user_token_account,
            amount,
            data,
        } => {
            let result = spl_token_deposit(
                l1_token,
                l2_token,
                receiver_twine_address,
                user_token_account,
                amount,
                data,
            );
            println!("Result: {:?}", result);
        }
        Commands::ForcedNativeWithdrawal {
            l1_token,
            l2_token,
            from_twine_address,
            l1_receiver,
            privkey,
            amount,
        } => {
            let result = forced_native_withdrawal(
                l1_token,
                l2_token,
                from_twine_address,
                l1_receiver,
                privkey,
                amount,
            );
            println!("Result: {:?}", result);
        }
        Commands::ForcedSplWithdrawal {
            l1_token,
            l2_token,
            from_twine_address,
            privkey,
            user_token_account,
            amount,
        } => {
            let result = forced_spl_withdrawal(
                l1_token,
                l2_token,
                from_twine_address,
                privkey,
                user_token_account,
                amount,
            );
            println!("Result: {:?}", result);
        }
        Commands::ExecuteNativeL2Withdrawal {
            receiver,
            public_values,
            proof,
        } => match execute_native_l2_withdrawal(receiver, public_values, proof) {
            Ok(_) => {
                println!("L2 originated withdrawal successful");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ExecuteSplL2Withdrawal {
            spl_token_pubkey,
            l1_receiver_address,
            public_values,
            execution_proof,
        } => match execute_spl_l2_withdrawal(
            spl_token_pubkey,
            l1_receiver_address,
            public_values,
            execution_proof,
        ) {
            Ok(_) => {
                println!("L2 originated withdrawal successful");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessNativeRefund {
            message_nonce,
            receiver,
            public_values,
            proof,
        } => match process_native_l1_refund(message_nonce, receiver, public_values, proof) {
            Ok(_) => {
                println!("refund successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessSplRefund {
            l1_token,
            l1_receiver_address,
            message_nonce,
            public_values,
            proof,
        } => match process_spl_l1_refund(
            l1_token,
            l1_receiver_address,
            message_nonce,
            public_values,
            proof,
        ) {
            Ok(_) => {
                println!("refund successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessNativeForcedWithdrawal {
            message_nonce,
            receiver,
            public_values,
            proof,
        } => {
            match process_native_l1_forced_withdrawal(message_nonce, receiver, public_values, proof)
            {
                Ok(_) => {
                    println!("forced withdrawal successfull");
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        }
        Commands::ProcessSplForcedWithdrawal {
            l1_token,
            l1_receiver_address,
            message_nonce,
            public_values,
            proof,
        } => match process_spl_l1_forced_withdrawal(
            l1_token,
            l1_receiver_address,
            message_nonce,
            public_values,
            proof,
        ) {
            Ok(_) => {
                println!("refund successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
    }
    Ok(())
}
