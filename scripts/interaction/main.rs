use action_commands::Cli;
use clap::*;
pub mod action_commands;
pub mod actions;
pub mod operations;
pub mod utils;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    actions::handle_command(cli.command)
}
