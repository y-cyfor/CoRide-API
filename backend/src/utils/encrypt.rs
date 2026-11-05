use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};

/// Prefix marking a ciphertext produced by the current format.
///
/// Legacy rows (written before encryption or by older versions) have no prefix and
/// are treated as plaintext when they cannot be decrypted.
const PREFIX: &str = "enc:v1:";

/// Result of attempting to read a stored secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecryptOutcome {
    /// Successfully decrypted with one of the configured keys.
    Decrypted(String),
    /// Not encrypted (legacy plaintext), returned as-is.
    Plaintext(String),
    /// Stored value looks like ciphertext but no configured key could open it.
    Failed(String),
}

impl DecryptOutcome {
    /// Best-effort plaintext: ciphertext stays as-is when it cannot be decrypted.
    pub fn into_string(self) -> String {
        match self {
            DecryptOutcome::Decrypted(v) | DecryptOutcome::Plaintext(v) => v,
            DecryptOutcome::Failed(v) => v,
        }
    }

    pub fn is_usable(&self) -> bool {
        !matches!(self, DecryptOutcome::Failed(_))
    }
}

/// A set of AES-256-GCM keys.
///
/// The first key is the *primary* key used for new encryptions; every key is tried
/// when decrypting so operators can rotate the secret without losing existing data.
#[derive(Clone)]
pub struct KeyRing {
    keys: Vec<[u8; 32]>,
}

impl std::fmt::Debug for KeyRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyRing")
            .field("keys", &self.keys.len())
            .finish()
    }
}

impl KeyRing {
    /// Build a ring from raw secrets. Empty/blank entries are ignored.
    ///
    /// Returns `None` when no usable secret was supplied; callers should then
    /// fall back to a default secret.
    pub fn from_secrets<S: AsRef<str>>(secrets: &[S]) -> Option<Self> {
        let mut keys: Vec<[u8; 32]> = Vec::new();
        for secret in secrets {
            let secret = secret.as_ref().trim();
            if secret.is_empty() {
                continue;
            }
            let key = derive_key(secret);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        if keys.is_empty() {
            None
        } else {
            Some(Self { keys })
        }
    }

    /// Build a ring from a single secret.
    pub fn from_secret<S: AsRef<str>>(secret: S) -> Self {
        Self::from_secrets(&[secret]).expect("secret must not be empty")
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Encrypt with the primary key, returning a prefixed base64 payload.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.keys[0]).map_err(|e| e.to_string())?;

        // Cryptographically secure random 12-byte nonce.
        let mut nonce_bytes = [0u8; 12];
        getrandom::getrandom(&mut nonce_bytes).map_err(|e: getrandom::Error| e.to_string())?;
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| e.to_string())?;

        let mut combined = nonce.to_vec();
        combined.extend(ciphertext);
        Ok(format!("{PREFIX}{}", STANDARD.encode(combined)))
    }

    /// Decrypt a stored value, trying every key in the ring.
    pub fn decrypt(&self, stored: &str) -> DecryptOutcome {
        let trimmed = stored.trim();
        if trimmed.is_empty() {
            return DecryptOutcome::Plaintext(stored.to_string());
        }

        let prefixed = trimmed.starts_with(PREFIX);
        let payload = if prefixed {
            &trimmed[PREFIX.len()..]
        } else {
            trimmed
        };

        // A bare base64 blob could still be legacy ciphertext (pre-prefix builds).
        let Ok(combined) = STANDARD.decode(payload) else {
            return if prefixed {
                DecryptOutcome::Failed(stored.to_string())
            } else {
                DecryptOutcome::Plaintext(stored.to_string())
            };
        };

        if combined.len() < 13 {
            return if prefixed {
                DecryptOutcome::Failed(stored.to_string())
            } else {
                DecryptOutcome::Plaintext(stored.to_string())
            };
        }

        let (nonce, ciphertext) = combined.split_at(12);
        let nonce: [u8; 12] = match nonce.try_into() {
            Ok(n) => n,
            Err(_) => return DecryptOutcome::Failed(stored.to_string()),
        };

        for key in &self.keys {
            let Ok(cipher) = Aes256Gcm::new_from_slice(key) else {
                continue;
            };
            if let Ok(plaintext) = cipher.decrypt(&Nonce::from(nonce), ciphertext)
                && let Ok(text) = String::from_utf8(plaintext) {
                    return DecryptOutcome::Decrypted(text);
                }
        }

        if prefixed {
            DecryptOutcome::Failed(stored.to_string())
        } else {
            // Unprefixed and undecryptable => legacy plaintext value.
            DecryptOutcome::Plaintext(stored.to_string())
        }
    }
}

/// Derive a 32-byte key from a string (SHA-256).
pub fn derive_key(secret: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_primary_key() {
        let ring = KeyRing::from_secret("secret-a");
        let encrypted = ring.encrypt(r#"["sk-abc","sk-def"]"#).unwrap();
        assert!(encrypted.starts_with(PREFIX));
        assert_eq!(
            ring.decrypt(&encrypted),
            DecryptOutcome::Decrypted(r#"["sk-abc","sk-def"]"#.to_string())
        );
    }

    #[test]
    fn ciphertext_is_not_stable_between_calls() {
        let ring = KeyRing::from_secret("secret-a");
        let a = ring.encrypt("same").unwrap();
        let b = ring.encrypt("same").unwrap();
        assert_ne!(a, b, "random nonce must make ciphertext unique");
    }

    #[test]
    fn rotation_can_still_read_old_keys() {
        let old = KeyRing::from_secret("old-secret");
        let encrypted = old.encrypt("payload").unwrap();

        // New primary + old key retained in the ring.
        let rotated = KeyRing::from_secrets(&["new-secret", "old-secret"]).unwrap();
        assert_eq!(
            rotated.decrypt(&encrypted),
            DecryptOutcome::Decrypted("payload".to_string())
        );

        // Encrypting with the rotated ring uses the new primary.
        let reencrypted = rotated.encrypt("payload").unwrap();
        assert_eq!(
            rotated.decrypt(&reencrypted),
            DecryptOutcome::Decrypted("payload".to_string())
        );
        assert_eq!(
            old.decrypt(&reencrypted),
            DecryptOutcome::Failed(reencrypted.clone()),
            "old key alone must not open data encrypted with the new primary"
        );
    }

    #[test]
    fn legacy_plaintext_is_passed_through() {
        let ring = KeyRing::from_secret("secret-a");
        assert_eq!(
            ring.decrypt(r#"["sk-plain"]"#),
            DecryptOutcome::Plaintext(r#"["sk-plain"]"#.to_string())
        );
    }

    #[test]
    fn legacy_unprefixed_ciphertext_still_decrypts() {
        let ring = KeyRing::from_secret("secret-a");
        // Simulate a value written by a pre-prefix build.
        let prefixed = ring.encrypt("payload").unwrap();
        let legacy = prefixed.trim_start_matches(PREFIX).to_string();
        assert_eq!(
            ring.decrypt(&legacy),
            DecryptOutcome::Decrypted("payload".to_string())
        );
    }

    #[test]
    fn wrong_key_reports_failure_for_prefixed_value() {
        let good = KeyRing::from_secret("good");
        let bad = KeyRing::from_secret("bad");
        let encrypted = good.encrypt("payload").unwrap();
        assert!(matches!(bad.decrypt(&encrypted), DecryptOutcome::Failed(_)));
    }
}
