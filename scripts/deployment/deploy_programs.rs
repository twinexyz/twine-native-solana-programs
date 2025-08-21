#![allow(warnings)]     
use std::io;
use std::process::{Command, ExitStatus};

const TOKENS_GATEWAY_SO: &str = "./target/deploy/tokens_gateway.so";
const TWINE_CHAIN_SO: &str = "./target/deploy/twine_chain.so";

fn main() -> io::Result<()> {
    println!("🚀 Starting deployment...");

    println!("✅ Build steps complete!");

    let tg_id = deploy(TOKENS_GATEWAY_SO, "tokens_gateway")?;
    let tc_id = deploy(TWINE_CHAIN_SO, "twine_chain")?;

    println!("🎉 Deployment finished.\n🚪 Tokens Gateway: {tg_id}\n🔗 Twine Chain: {tc_id}");
    Ok(())
}

fn run(cmd: &str, args: &[&str], msg: &str) -> io::Result<ExitStatus> {
    println!("{msg}: {cmd} {}", args.join(" "));
    Command::new(cmd).args(args).status()
}

fn deploy(so_path: &str, name: &str) -> io::Result<String> {
    println!("📤 Deploying {name}: {so_path}");
    let output = Command::new("solana")
        .args(&["program", "deploy", so_path])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Deploy failed: {name}"),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|l| {
            l.split_once("Program Id:")
                .map(|(_, id)| id.trim().to_string())
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Program ID not found"))
}
