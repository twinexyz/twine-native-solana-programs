use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;

const OAPP_SO: &str = "./target/deploy/oapp.so";
const TWINE_CHAIN_SO: &str = "./target/deploy/twine_chain.so";
const TOKENS_GATEWAY_SO: &str = "./target/deploy/tokens_gateway.so";
const TOKENS_GATEWAY_KEYPAIR: &str = "./target/deploy/tokens_gateway-keypair.json";

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

    // Deploy tokens gateway using buffer with specific program-id
    let tg_id = deploy_with_buffer(TOKENS_GATEWAY_SO, "tokens_gateway", &url, &keypair_path)?;

    // Deploy twine chain using the original method
    let tc_id = deploy(TWINE_CHAIN_SO, "twine_chain", &url, &keypair_path)?;

    // Deploy OApp
    let oapp_id = deploy(OAPP_SO, "oapp", &url, &keypair_path)?;

    println!("🎉 Deployment finished.\n🚪 Tokens Gateway: {tg_id}");
    println!("🎉 Deployment finished.\n🔗 Twine Chain: {tc_id}");
    println!("🎉 Deployment finished.\n🅾️  LZ OApp: {oapp_id}");

    Ok(())
}

fn deploy_with_buffer(
    so_path: &str,
    name: &str,
    url: &str,
    keypair_path: &PathBuf,
) -> io::Result<String> {
    println!("📤 Deploying {name} with buffer: {so_path}");

    // Step 1: Write program to buffer
    let buffer_args = vec![
        "program",
        "write-buffer",
        so_path,
        "--url",
        url,
        "--commitment",
        "confirmed",
        "--keypair",
        keypair_path.to_str().expect("Invalid keypair path"),
    ];

    eprintln!("🔧 Exec: solana {}", buffer_args.join(" "));

    let buffer_output = Command::new("solana").args(&buffer_args).output()?;

    if !buffer_output.status.success() {
        let stderr = String::from_utf8_lossy(&buffer_output.stderr);
        let stdout = String::from_utf8_lossy(&buffer_output.stdout);
        eprintln!("❌ Buffer creation failed for {name}");
        eprintln!("Exit code: {:?}", buffer_output.status.code());
        if !stderr.is_empty() {
            eprintln!("--- STDERR ---\n{}", stderr);
        }
        if !stdout.is_empty() {
            eprintln!("--- STDOUT ---\n{}", stdout);
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Buffer creation failed: {name}. See logs above."),
        ));
    }

    let buffer_stdout = String::from_utf8_lossy(&buffer_output.stdout);
    let buffer_address = buffer_stdout
        .lines()
        .find_map(|l| {
            l.split_once("Buffer:")
                .map(|(_, addr)| addr.trim().to_string())
        })
        .ok_or_else(|| {
            eprintln!("⚠️  Buffer address not found in output for {name}");
            eprintln!("Full output:\n{buffer_stdout}");
            io::Error::new(io::ErrorKind::Other, "Buffer address not found")
        })?;

    println!("📝 Buffer created: {buffer_address}");

    // Step 2: Deploy program from buffer with specific program-id and compute unit price
    let deploy_args = vec![
        "program",
        "deploy",
        "--url",
        url,
        "--keypair",
        keypair_path.to_str().expect("Invalid keypair path"),
        "--buffer",
        &buffer_address,
        "--program-id",
        TOKENS_GATEWAY_KEYPAIR,
        "--with-compute-unit-price",
        "1000000",
        "--commitment",
        "confirmed",
    ];

    eprintln!("🔧 Exec: solana {}", deploy_args.join(" "));

    let deploy_output = Command::new("solana").args(&deploy_args).output()?;

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        let stdout = String::from_utf8_lossy(&deploy_output.stdout);
        eprintln!("❌ Deploy from buffer failed for {name}");
        eprintln!("Exit code: {:?}", deploy_output.status.code());
        if !stderr.is_empty() {
            eprintln!("--- STDERR ---\n{}", stderr);
        }
        if !stdout.is_empty() {
            eprintln!("--- STDOUT ---\n{}", stdout);
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Deploy from buffer failed: {name}. See logs above."),
        ));
    }

    let deploy_stdout = String::from_utf8_lossy(&deploy_output.stdout);
    let program_id = deploy_stdout
        .lines()
        .find_map(|l| {
            l.split_once("Program Id:")
                .map(|(_, id)| id.trim().to_string())
        })
        .ok_or_else(|| {
            eprintln!("⚠️  Program ID not found in output for {name}");
            eprintln!("Full output:\n{deploy_stdout}");
            io::Error::new(io::ErrorKind::Other, "Program ID not found")
        })?;
    Ok(program_id)
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
