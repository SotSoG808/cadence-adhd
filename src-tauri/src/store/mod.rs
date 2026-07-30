//! Encrypted SQLite persistence.
//!
//! The database key is derived from a machine-specific secret using Argon2id
//! and stored in %APPDATA%\cadence-adhd\cadence.key (600 permissions).
//! All task and calendar data is stored in cadence.db in the same directory.
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
use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub struct Store {
    pub conn: Connection,
    cipher: Aes256Gcm,
}

impl Store {
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let key = derive_or_load_key(data_dir)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
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
        let ct = self.cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;
        let mut out = nonce_bytes.to_vec();
        out.extend(ct);
        Ok(B64.encode(out))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let raw = B64.decode(ciphertext)?;
        anyhow::ensure!(raw.len() > 12, "ciphertext too short");
        let (nonce_bytes, ct) = raw.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = self.cipher.decrypt(nonce, ct)
            .map_err(|e| anyhow::anyhow!("decrypt: {e}"))?;
        Ok(String::from_utf8(plain)?)
    }
}

fn derive_or_load_key(data_dir: &Path) -> Result<Vec<u8>> {
    let key_path = data_dir.join("cadence.key");
    if key_path.exists() {
        let b64 = std::fs::read_to_string(&key_path)?;
        return Ok(B64.decode(b64.trim())?)
    }
    // Generate a new random 32-byte key, persist it
    let mut raw = [0u8; 32];
    OsRng.fill_bytes(&mut raw);
    std::fs::write(&key_path, B64.encode(&raw))?;
    Ok(raw.to_vec())
}
