use action_commands::Cli;
use clap::*;
pub mod utils;
pub mod actions;
pub mod operations;
pub mod action_commands;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    actions::handle_command(cli.command)
}
