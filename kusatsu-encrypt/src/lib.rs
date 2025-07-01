use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::ZeroizeOnDrop;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid key format")]
    InvalidKeyFormat,
    #[error("Invalid nonce length")]
    InvalidNonceLength,
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

/// A secure encryption key that zeroes itself when dropped
#[derive(Clone, ZeroizeOnDrop, Debug)]
pub struct EncryptionKey {
    key: [u8; 32], // 256-bit key for AES-256
}

impl EncryptionKey {
    /// Generate a new random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    /// Create an encryption key from a base64-encoded string
    pub fn from_base64(encoded: &str) -> Result<Self, EncryptionError> {
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(encoded)?;
        if decoded.len() != 32 {
            println!("Invalid key format: {}", encoded);
            return Err(EncryptionError::InvalidKeyFormat);
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        Ok(Self { key })
    }

    /// Create an encryption key from a URL-encoded string (alias for from_base64)
    pub fn from_url_encoded(encoded: &str) -> Result<Self, EncryptionError> {
        Self::from_base64(encoded)
    }

    /// Convert the encryption key to a base64-encoded string for URL anchors
    pub fn to_base64(&self) -> String {
        general_purpose::URL_SAFE_NO_PAD.encode(self.key)
    }

    /// Get the raw key bytes (use with caution)
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

/// Encrypted data with nonce
#[derive(Serialize, Deserialize, Clone)]
pub struct EncryptedData {
    /// The encrypted content
    pub ciphertext: Vec<u8>,
    /// The nonce used for encryption (stored alongside the data)
    pub nonce: Vec<u8>,
}

/// Generic encryption and decryption functions
pub struct Encryption;

impl Encryption {
    /// Encrypt arbitrary data with a given key
    pub fn encrypt(data: &[u8], key: &EncryptionKey) -> Result<EncryptedData, EncryptionError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));

        // Generate random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt data
        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|_| EncryptionError::EncryptionFailed)?;

        Ok(EncryptedData {
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    /// Decrypt data with a given key
    pub fn decrypt(
        encrypted_data: &EncryptedData,
        key: &EncryptionKey,
    ) -> Result<Vec<u8>, EncryptionError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));

        // Reconstruct nonce
        if encrypted_data.nonce.len() != 12 {
            return Err(EncryptionError::InvalidNonceLength);
        }
        let nonce = Nonce::from_slice(&encrypted_data.nonce);

        // Decrypt data
        let plaintext = cipher
            .decrypt(nonce, encrypted_data.ciphertext.as_ref())
            .map_err(|_| EncryptionError::DecryptionFailed)?;

        Ok(plaintext)
    }

    /// Encrypt data with a specific nonce (use with caution - nonces should be unique)
    pub fn encrypt_with_nonce(
        data: &[u8],
        key: &EncryptionKey,
        nonce: &[u8; 12],
    ) -> Result<EncryptedData, EncryptionError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_bytes()));
        let nonce_obj = Nonce::from_slice(nonce);

        let ciphertext = cipher
            .encrypt(nonce_obj, data)
            .map_err(|_| EncryptionError::EncryptionFailed)?;

        Ok(EncryptedData {
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    /// Generate a new encryption key and return it as a base64 string
    pub fn generate_key_string() -> String {
        EncryptionKey::generate().to_base64()
    }

    /// Generate a random nonce
    pub fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}

// Convenience functions for common use cases
impl Encryption {
    /// Encrypt a string and return base64-encoded result
    pub fn encrypt_string(text: &str, key: &EncryptionKey) -> Result<String, EncryptionError> {
        let encrypted = Self::encrypt(text.as_bytes(), key)?;
        let combined = [&encrypted.nonce[..], &encrypted.ciphertext[..]].concat();
        Ok(general_purpose::STANDARD.encode(combined))
    }

    /// Decrypt a base64-encoded string
    pub fn decrypt_string(encoded: &str, key: &EncryptionKey) -> Result<String, EncryptionError> {
        let combined = general_purpose::STANDARD.decode(encoded)?;

        if combined.len() < 12 {
            return Err(EncryptionError::InvalidNonceLength);
        }

        let encrypted_data = EncryptedData {
            nonce: combined[..12].to_vec(),
            ciphertext: combined[12..].to_vec(),
        };

        let decrypted = Self::decrypt(&encrypted_data, key)?;
        String::from_utf8(decrypted).map_err(|_| EncryptionError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation_and_conversion() {
        let key = EncryptionKey::generate();
        let base64_key = key.to_base64();
        let restored_key = EncryptionKey::from_base64(&base64_key).unwrap();

        assert_eq!(key.as_bytes(), restored_key.as_bytes());
    }

    #[test]
    fn test_encryption_decryption() {
        let key = EncryptionKey::generate();
        let original_data = b"Hello, World! This is a test message.";

        // Encrypt
        let encrypted = Encryption::encrypt(original_data, &key).unwrap();

        // Decrypt
        let decrypted_data = Encryption::decrypt(&encrypted, &key).unwrap();

        assert_eq!(original_data, &decrypted_data[..]);
    }

    #[test]
    fn test_string_encryption_decryption() {
        let key = EncryptionKey::generate();
        let original_text = "Hello, World! This is a test message.";

        // Encrypt
        let encrypted = Encryption::encrypt_string(original_text, &key).unwrap();

        // Decrypt
        let decrypted_text = Encryption::decrypt_string(&encrypted, &key).unwrap();

        assert_eq!(original_text, decrypted_text);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let data = b"Secret data";

        let encrypted = Encryption::encrypt(data, &key1).unwrap();

        // Should fail with wrong key
        assert!(Encryption::decrypt(&encrypted, &key2).is_err());
    }

    #[test]
    fn test_nonce_uniqueness() {
        let key = EncryptionKey::generate();
        let data = b"Same data";

        let encrypted1 = Encryption::encrypt(data, &key).unwrap();
        let encrypted2 = Encryption::encrypt(data, &key).unwrap();

        // Same data encrypted twice should have different nonces
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        // But both should decrypt to the same data
        assert_eq!(Encryption::decrypt(&encrypted1, &key).unwrap(), data);
        assert_eq!(Encryption::decrypt(&encrypted2, &key).unwrap(), data);
    }
}
