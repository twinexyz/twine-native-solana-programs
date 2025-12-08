use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

const OAPP_PROGRAM_SO_PATH: &str = "./target/deploy/oapp.so";

fn get_default_keypair_path() -> PathBuf {
    let mut keypair_path = dirs::home_dir().expect("Could not get home directory");
    keypair_path.push(".config/solana/id.json");
    keypair_path
}

fn get_program_id_from_user() -> io::Result<String> {
    print!("🔑 Please enter the OAPP Program ID: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let program_id = input.trim().to_string();

    if program_id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Program ID cannot be empty",
        ));
    }

    Ok(program_id)
}

fn main() -> io::Result<()> {
    println!("🚀 Starting OAPP program update process...");

    // Optional env overrides
    let url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());

    // Use environment variable if provided, otherwise fall back to default path
    let keypair_path = env::var("SOLANA_KEYPAIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_default_keypair_path());

    // Get program ID from user input
    let program_id = get_program_id_from_user()?;

    println!("Using cluster: {url}");
    println!("Using keypair: {}", keypair_path.display());
    println!("Using program ID: {program_id}");

    update_program_with_buffer(
        OAPP_PROGRAM_SO_PATH,
        "oapp",
        &url,
        &keypair_path,
        &program_id,
    )?;

    println!("🎉 Update finished.");

    Ok(())
}

fn update_program_with_buffer(
    so_path: &str,
    name: &str,
    url: &str,
    keypair_path: &PathBuf,
    program_id: &str,
) -> io::Result<()> {
    println!("📤 Updating {name} with buffer: {so_path}");

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

    // Step 2: Deploy program from buffer using user-provided program ID
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
        program_id,
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

    // Extract and display the program ID from output
    let deployed_program_id = deploy_stdout
        .lines()
        .find_map(|l| {
            l.split_once("Program Id:")
                .map(|(_, id)| id.trim().to_string())
        })
        .unwrap_or_else(|| "Program ID not found in output".to_string());

    println!("🎉 Program updated successfully!");
    println!("🚪 Tokens Gateway Program ID: {deployed_program_id}");

    Ok(())
}
