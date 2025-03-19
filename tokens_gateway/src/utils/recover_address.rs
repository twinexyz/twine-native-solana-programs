use sha3::{Digest, Keccak256};
use solana_secp256k1_recover::secp256k1_recover;
use solana_program::{program_error::ProgramError,msg};
use twine_chain::core::state::ForcedWithdrawMessageInfo;

pub(crate) fn recover_address(
    withdraw_info:ForcedWithdrawMessageInfo,
    signature: Vec<u8>,
) ->  Result<String, ProgramError> {
    // Ensure the signature length is exactly 65 bytes (64 bytes signature + 1 byte v)
    if signature.len() != 65 {
        return Err(ProgramError::InvalidArgument.into());
    }
    // preparing the message that was signed
    let message = withdraw_info.abi_encode_packed();
    let mut message_hasher = Keccak256::new();
    message_hasher.update(&message);
    let message_hash = message_hasher.finalize();
    let message_signed: [u8; 32] = message_hash.into();

    // Extracting signature and recovery key from given signature
    let (signature, v) = signature.split_at(64);

    {
        let signature = libsecp256k1::Signature::parse_standard_slice(&signature)
            .map_err(|_| ProgramError::InvalidArgument)?;

        if signature.s.is_high() {
            msg!("signature with high-s value");
            return Err(ProgramError::InvalidArgument);
        }
    }

    let mut recovery_id = *v.first().unwrap();
    if recovery_id > 3 {
        recovery_id = if recovery_id == 27 { 0 } else { 1 };
    }

    // Recovering the singer's publicKey
    let recovered_pubkey = secp256k1_recover(&message_signed, recovery_id, signature)
        .map_err(|_| ProgramError::InvalidArgument)?;

    // Extracting the address form recovered publick key
    let pubkey_bytes = recovered_pubkey.to_bytes();
    let mut pubkey_hasher = Keccak256::new();
    pubkey_hasher.update(&pubkey_bytes);
    let pubkey_hash = pubkey_hasher.finalize();

    let twine_address = &pubkey_hash[12..];
    let recovered_address = format!("0x{}", hex::encode(twine_address));
    Ok(recovered_address)
}