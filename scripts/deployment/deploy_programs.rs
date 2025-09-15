use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;

const TOKENS_GATEWAY_SO: &str = "./target/deploy/tokens_gateway.so";
const TWINE_CHAIN_SO: &str = "./target/deploy/twine_chain.so";

fn get_default_keypair_path() -> PathBuf {
    let mut keypair_path = dirs::home_dir().expect("Could not get home directory");
    keypair_path.push(".config/solana/id.json");
    keypair_path
}

fn main() -> io::Result<()> {
    println!("🚀 Starting deployment...");

    // Optional env overrides
    // SOLANA_RPC_URL (e.g., "http://127.0.0.1:8899" or "https://api.devnet.solana.com")
    // SOLANA_KEYPAIR (e.g., "/home/me/.config/solana/id.json")
    let url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());

    // Use environment variable if provided, otherwise fall back to default path
    let keypair_path = env::var("SOLANA_KEYPAIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_default_keypair_path());

    println!("Using cluster: {url}");
    println!("Using keypair: {}", keypair_path.display());

    println!("✅ Build steps complete!");

    let tg_id = deploy(TOKENS_GATEWAY_SO, "tokens_gateway", &url, &keypair_path)?;
    let tc_id = deploy(TWINE_CHAIN_SO, "twine_chain", &url, &keypair_path)?;

    println!("🎉 Deployment finished.\n🚪 Tokens Gateway: {tg_id}");
    println!("🎉 Deployment finished.\n🔗 Twine Chain: {tc_id}");

    Ok(())
}

fn deploy(so_path: &str, name: &str, url: &str, keypair_path: &PathBuf) -> io::Result<String> {
    println!("📤 Deploying {name}: {so_path}");

    let args = vec![
        "program",
        "deploy",
        so_path,
        "--url",
        url,
        "--commitment",
        "confirmed",
        "--keypair",
        keypair_path.to_str().expect("Invalid keypair path"),
    ];

    // Show the exact command for debugging
    eprintln!("🔧 Exec: solana {}", args.join(" "));

    let output = Command::new("solana").args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("❌ Deploy failed for {name}");
        eprintln!("Exit code: {:?}", output.status.code());
        if !stderr.is_empty() {
            eprintln!("--- STDERR ---\n{}", stderr);
        }
        if !stdout.is_empty() {
            eprintln!("--- STDOUT ---\n{}", stdout);
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Deploy failed: {name}. See logs above."),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|l| {
            l.split_once("Program Id:")
                .map(|(_, id)| id.trim().to_string())
        })
        .ok_or_else(|| {
            eprintln!("⚠️  Program ID not found in output for {name}");
            eprintln!("Full output:\n{stdout}");
            io::Error::new(io::ErrorKind::Other, "Program ID not found")
        })
}
