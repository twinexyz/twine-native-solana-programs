use action_commands::Cli;
use clap::*;
pub mod utils;
pub mod actions;
pub mod operations;
pub mod action_commands;
pub mod tokens_gateway_client;
pub mod twine_chain_client;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    actions::handle_command(cli.command)
}
