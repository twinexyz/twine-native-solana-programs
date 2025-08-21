#![allow(warnings)]     
use std::process::{Command, ExitStatus};
use std::io::{self, Write};

const TWINE_CHAIN_PROGRAM_SO_PATH: &str = "./target/deploy/twine_chain.so";

fn main() -> io::Result<()> {
    println!("🚀 Starting twine_chain program update process...");


    print!("💡 Enter the twine_chain Program ID (base58, as on-chain): ");
    io::stdout().flush()?; 
    let mut program_id = String::new();
    io::stdin().read_line(&mut program_id)?;
    let program_id = program_id.trim();

    deploy_program(TWINE_CHAIN_PROGRAM_SO_PATH, "twine_chain", program_id)?;

    println!("🎉 Update finished.");
    println!("🚪 Twine Chain Program ID (Updated): {}", program_id);

    Ok(())
}

fn deploy_program(so_path: &str, program_name: &str, program_id: &str) -> io::Result<()> {
    println!(
        "📤 Updating {} on Solana: {} (Program ID: {})",
        program_name, so_path, program_id
    );
    let output = Command::new("solana")
        .args(&["program", "deploy", so_path, "--program-id", program_id])
        .output()?;

    if !output.status.success() {
        eprintln!("❌ Failed to deploy {}!", program_name);
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    println!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

fn run_cmd(cmd: &str, args: &[&str], msg: &str) -> io::Result<ExitStatus> {
    println!("{}: {} {}", msg, cmd, args.join(" "));

    let status = Command::new(cmd)
        .args(args)
        .status()?;

    if !status.success() {
        eprintln!("❌ Command failed: {} {}", cmd, args.join(" "));
        return Err(io::Error::new(io::ErrorKind::Other, "Command execution failed"));
    }

    Ok(status)
}
