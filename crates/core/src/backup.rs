//! Password-protected backup files (`.gasbackup`), portable between machines.
//!
//! JSON envelope: PBKDF2-HMAC-SHA256 (200k rounds, 16-byte salt) derives an
//! AES-256-GCM key; the accounts JSON is the ciphertext. The password is never
//! stored.

use crate::model::Account;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};

const FORMAT: &str = "game-account-switcher-backup";
const VERSION: u32 = 1;
const ROUNDS: u32 = 200_000;

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    kdf: String,
    rounds: u32,
    salt: String,
    nonce: String,
    data: String,
}

fn derive_key(password: &str, salt: &[u8], rounds: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), salt, rounds, &mut key);
    key
}

fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).map_err(|e| format!("Không tạo được số ngẫu nhiên: {e}"))?;
    Ok(b)
}

pub fn export(accounts: &[Account], password: &str) -> Result<String, String> {
    if password.chars().count() < 6 {
        return Err("Mật khẩu sao lưu cần ít nhất 6 ký tự".into());
    }
    let salt: [u8; 16] = random()?;
    let nonce: [u8; 12] = random()?;
    let key = derive_key(password, &salt, ROUNDS);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let plain = serde_json::to_vec(accounts).map_err(|e| e.to_string())?;
    let data = cipher
        .encrypt(Nonce::from_slice(&nonce), plain.as_slice())
        .map_err(|_| "Mã hoá thất bại".to_string())?;
    let env = Envelope {
        format: FORMAT.into(),
        version: VERSION,
        kdf: "pbkdf2-sha256".into(),
        rounds: ROUNDS,
        salt: B64.encode(salt),
        nonce: B64.encode(nonce),
        data: B64.encode(data),
    };
    serde_json::to_string_pretty(&env).map_err(|e| e.to_string())
}

pub fn import(text: &str, password: &str) -> Result<Vec<Account>, String> {
    let env: Envelope =
        serde_json::from_str(text).map_err(|_| "Đây không phải file sao lưu hợp lệ".to_string())?;
    if env.format != FORMAT || env.version != VERSION || env.kdf != "pbkdf2-sha256" {
        return Err("File sao lưu không đúng định dạng hoặc phiên bản".into());
    }
    if env.rounds < 10_000 || env.rounds > 10_000_000 {
        return Err("File sao lưu có tham số không hợp lệ".into());
    }
    let dec = |s: &str| B64.decode(s.as_bytes()).map_err(|_| "File sao lưu bị hỏng".to_string());
    let salt = dec(&env.salt)?;
    let nonce = dec(&env.nonce)?;
    let data = dec(&env.data)?;
    if nonce.len() != 12 {
        return Err("File sao lưu bị hỏng".into());
    }
    let key = derive_key(password, &salt, env.rounds);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce), data.as_slice())
        .map_err(|_| "Sai mật khẩu hoặc file đã bị sửa".to_string())?;
    serde_json::from_slice(&plain).map_err(|_| "Nội dung sao lưu không đọc được".to_string())
}

/// Adds `incoming` accounts that don't already exist (same launcher + username).
/// Returns how many were added.
pub fn merge(into: &mut Vec<Account>, incoming: Vec<Account>) -> usize {
    let mut added = 0;
    for mut acc in incoming {
        let exists = into.iter().any(|a| {
            a.launcher == acc.launcher && a.username.eq_ignore_ascii_case(&acc.username)
        });
        if exists {
            continue;
        }
        if into.iter().any(|a| a.id == acc.id) {
            acc.id = crate::model::new_id();
        }
        into.push(acc);
        added += 1;
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Game, Launcher};

    fn sample() -> Vec<Account> {
        vec![Account {
            id: "a1".into(),
            launcher: Launcher::Riot,
            label: "Main".into(),
            username: "player".into(),
            password: "hunter2".into(),
            note: "".into(),
            default_game: Some(Game::League),
            last_used: None,
        }]
    }

    #[test]
    fn roundtrip_and_wrong_password() {
        let text = export(&sample(), "correct horse").unwrap();
        assert!(!text.contains("hunter2"));
        assert_eq!(import(&text, "correct horse").unwrap(), sample());
        assert!(import(&text, "wrong pass").is_err());
    }

    #[test]
    fn short_password_rejected() {
        assert!(export(&sample(), "12345").is_err());
    }

    #[test]
    fn merge_skips_existing() {
        let mut list = sample();
        let mut other = sample();
        other.push(Account { id: "a1".into(), username: "second".into(), ..sample()[0].clone() });
        assert_eq!(merge(&mut list, other), 1);
        assert_eq!(list.len(), 2);
        assert_ne!(list[0].id, list[1].id);
    }
}
