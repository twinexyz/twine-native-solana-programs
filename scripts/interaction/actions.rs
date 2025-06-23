use crate::action_commands::Commands;
use crate::operations::{
    deposit_native_token::native_token_deposit, forced_native_withdrawal::forced_native_withdrawal,
    initialize_programs::initialize_twine_solana_programs, token_mapping::update_token_mapping,
    forced_spl_withdrawal::forced_spl_withdrawal,create_spl_token::create_spl_token,deposit_spl_token::spl_token_deposit,
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
        } => {
            let result = native_token_deposit(l1_token, l2_token, receiver_twine_address, amount);
            println!("Result: {:?}", result);
        }
        Commands::DepositSplToken {
            l1_token,
            l2_token,
            receiver_twine_address,
            user_token_account,
            amount,
        } => {
            let result = spl_token_deposit(l1_token, l2_token, receiver_twine_address, user_token_account,amount);
            println!("Result: {:?}", result);
        }
        Commands::ForcedNativeWithdrawal {
            l1_token,
            l2_token,
            from_twine_address,
            privkey,
            amount,
        } => {
            let result =
                forced_native_withdrawal(l1_token, l2_token, from_twine_address, privkey, amount);
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
            let result =
                forced_spl_withdrawal(l1_token, l2_token, from_twine_address, privkey,user_token_account, amount);
            println!("Result: {:?}", result);
        }
    }
    Ok(())
}
