use aes_gcm::{
    aead::Aead,
    Aes256Gcm, KeyInit, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};

/// Encrypt a string using AES-256-GCM.
/// Returns a base64-encoded string containing nonce + ciphertext.
pub fn encrypt(plaintext: &str, key_bytes: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;

    // Generate a random 12-byte nonce
    let nonce_bytes: [u8; 12] = std::array::from_fn(|_| fastrand::u8(0..=255));
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(|e| e.to_string())?;

    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    Ok(STANDARD.encode(&combined))
}

/// Decrypt a string that was encrypted with `encrypt`.
pub fn decrypt(encoded: &str, key_bytes: &[u8; 32]) -> Result<String, String> {
    let combined = STANDARD.decode(encoded).map_err(|e| e.to_string())?;
    if combined.len() < 13 {
        return Err("Invalid encrypted data".to_string());
    }

    let (nonce, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;
    let plaintext = cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|e| e.to_string())?;

    String::from_utf8(plaintext).map_err(|e| e.to_string())
}

/// Derive a 32-byte key from a string (using SHA-256 for key derivation).
pub fn derive_key(secret: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}
