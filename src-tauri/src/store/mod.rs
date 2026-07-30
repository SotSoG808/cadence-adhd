//! Encrypted SQLite persistence.
//!
//! The database key is derived from a per-install random secret using Argon2id
//! and stored in %APPDATA%\cadence-adhd\cadence.key. All task and calendar
//! data is stored in cadence.db in the same directory.
//!
//! The encryption layer wraps rusqlite: each TEXT/BLOB column that contains
//! personal data is AES-256-GCM encrypted at the application layer before
//! SQLite writes it. Metadata columns (ids, timestamps, enums) are plaintext
//! so the scheduler can query them without decrypting the whole table.

pub mod migrations;

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use aes_gcm::aead::rand_core::RngCore;
use anyhow::{anyhow, Context, Result};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rusqlite::Connection;
use std::path::Path;

pub struct Store {
    pub conn: Connection,
    cipher: Aes256Gcm,
}

impl Store {
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let key = derive_or_load_key(data_dir)?;
        if key.len() != 32 {
            return Err(anyhow!("derived key must be 32 bytes, got {}", key.len()));
        }
        let key = Key::<Aes256Gcm>::from_slice(&key);
        let cipher = Aes256Gcm::new(key);

        let db_path = data_dir.join("cadence.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("opening {}", db_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        migrations::run(&conn)?;

        Ok(Self { conn, cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ct = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("encrypt: {e}"))?;

        let mut out = nonce_bytes.to_vec();
        out.extend(ct);
        Ok(B64.encode(out))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let raw = B64.decode(ciphertext)?;
        anyhow::ensure!(raw.len() > 12, "ciphertext too short");

        let (nonce_bytes, ct) = raw.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plain = self
            .cipher
            .decrypt(nonce, ct)
            .map_err(|e| anyhow!("decrypt: {e}"))?;

        Ok(String::from_utf8(plain)?)
    }
}

fn derive_or_load_key(data_dir: &Path) -> Result<Vec<u8>> {
    let key_path = data_dir.join("cadence.key");

    if key_path.exists() {
        // Format: "<salt_b64>:<secret_b64>"
        let contents = std::fs::read_to_string(&key_path).context("read key file")?;
        let mut parts = contents.split(':');
        let salt_b64 = parts
            .next()
            .ok_or_else(|| anyhow!("missing salt in key file"))?;
        let secret_b64 = parts
            .next()
            .ok_or_else(|| anyhow!("missing secret in key file"))?;

        let salt = B64
            .decode(salt_b64.trim())
            .context("decode salt from key file")?;
        let secret = B64
            .decode(secret_b64.trim())
            .context("decode secret from key file")?;

        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(&secret, &salt, &mut key)
            .map_err(|e| anyhow!("argon2 hash_password_into (load): {e}"))?;

        return Ok(key.to_vec());
    }

    // First run: generate a random per-install secret and salt,
    // derive a key with Argon2id, and persist salt+secret.
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);

    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(&secret, &salt, &mut key)
        .map_err(|e| anyhow!("argon2 hash_password_into (derive): {e}"))?;

    let salt_b64 = B64.encode(&salt);
    let secret_b64 = B64.encode(&secret);
    std::fs::write(&key_path, format!("{}:{}", salt_b64, secret_b64)).context("write key file")?;

    Ok(key.to_vec())
}
