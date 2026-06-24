use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;
use std::env;
use std::sync::LazyLock;

static ENCRYPTION_KEY: LazyLock<Result<Key<Aes256Gcm>, String>> = LazyLock::new(|| {
    let key_str = env::var("API_ENCRYPTION_KEY").unwrap_or_default();
    let mut key_bytes = vec![0u8; 32];

    if key_str.len() == 64 && key_str.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(decoded) = hex::decode(&key_str) {
            key_bytes.copy_from_slice(&decoded);
            Ok(*Key::<Aes256Gcm>::from_slice(&key_bytes))
        } else {
            Err("API_ENCRYPTION_KEY must be a valid 64-character hex string".to_string())
        }
    } else {
        Err("API_ENCRYPTION_KEY must be exactly 64 characters of hex".to_string())
    }
});

pub struct EncryptionService;

impl EncryptionService {
    pub fn encryption_enabled() -> bool {
        static ENABLED: LazyLock<bool> = LazyLock::new(|| {
            env::var("API_PAYLOAD_ENCRYPTION_ENABLED").unwrap_or_default() == "true"
        });
        *ENABLED
    }

    fn get_key() -> anyhow::Result<Key<Aes256Gcm>> {
        ENCRYPTION_KEY
            .as_ref()
            .map(|k| *k)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn encrypt(plain_text: &str) -> anyhow::Result<String> {
        let key = Self::get_key()?;
        let cipher = Aes256Gcm::new(&key);

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes); // 96-bits; unique per message

        // In aes-gcm crate, encrypt returns a Vec<u8> which is Ciphertext + AuthTag concatenated
        let ciphertext = cipher
            .encrypt(nonce, plain_text.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failure: {:?}", e))?;

        // Combine IV (Nonce) + Ciphertext + AuthTag
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    pub fn decrypt(base64_payload: &str) -> anyhow::Result<String> {
        let decoded = BASE64
            .decode(base64_payload)
            .map_err(|e| anyhow::anyhow!("Base64 decode failure: {}", e))?;

        if decoded.len() < 28 {
            return Err(anyhow::anyhow!("Payload too short"));
        }

        let nonce_bytes = &decoded[0..12];
        let ciphertext_with_tag = &decoded[12..];

        let key = Self::get_key()?;
        let cipher = Aes256Gcm::new(&key);
        let nonce = Nonce::from_slice(nonce_bytes);

        // The decrypt function automatically verifies the AuthTag at the end of the ciphertext
        let plaintext = cipher
            .decrypt(nonce, ciphertext_with_tag)
            .map_err(|e| anyhow::anyhow!("Decryption failure or AuthTag mismatch: {:?}", e))?;

        Ok(String::from_utf8(plaintext)?)
    }
}
