//! One-time import from Riot Account Manager v3 (and the Java v1/v2 before it).
//!
//! Old data lives in `%LOCALAPPDATA%\RiotAccountManager\`:
//! * `accounts.dat` — JSON `[{username, password, region, note}]`, either
//!   `RAM2:` + Base64(DPAPI blob) (v3), or the Java format
//!   Base64(AES-256/ECB/PKCS7) keyed with SHA-256(MachineGuid).
//! * `config.json` — `riotClientPath`, `runAtStartup`, `autoClickLogin`.
//!
//! The old files are only read, never modified or deleted, so the old app keeps
//! working and a failed import can simply be retried.

use crate::model::{new_id, Account, Launcher};
use crate::paths::log_line;
use crate::settings::Settings;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const RAM2_PREFIX: &str = "RAM2:";
const MARKER: &str = "imported-from-v3";

#[derive(Debug, Deserialize)]
struct OldAccount {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    note: String,
}

pub fn old_data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("GAS_LEGACY_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    std::env::var("LOCALAPPDATA")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|base| PathBuf::from(base).join("RiotAccountManager"))
}

/// Result of a successful import, shown once in the UI.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ImportReport {
    pub accounts_added: usize,
    pub riot_path_imported: bool,
}

/// Imports old accounts and settings once. Returns `Ok(None)` when there was
/// nothing to do (already imported, or no old data).
pub fn import_once(
    accounts: &mut Vec<Account>,
    settings: &mut Settings,
) -> Result<Option<ImportReport>, String> {
    let marker = crate::paths::data_dir().join(MARKER);
    if marker.exists() {
        return Ok(None);
    }
    let Some(old_dir) = old_data_dir() else {
        return Ok(None);
    };
    let accounts_file = old_dir.join("accounts.dat");
    let config_file = old_dir.join("config.json");
    if !accounts_file.exists() && !config_file.exists() {
        return Ok(None);
    }

    let mut report = ImportReport::default();
    if config_file.exists() {
        report.riot_path_imported = apply_old_config(&config_file, settings);
    }
    let accounts_result = if accounts_file.exists() {
        read_old_accounts(&accounts_file).map(|old| {
            report.accounts_added = crate::backup::merge(accounts, convert(old));
        })
    } else {
        Ok(())
    };

    // Attempt only once, even on failure, so a broken old file doesn't nag on
    // every start. The old file is untouched and can still be opened by v3.
    crate::paths::ensure_data_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::write(&marker, crate::model::now_unix().to_string());
    match accounts_result {
        Ok(()) => {
            log_line(&format!(
                "imported from v3: {} account(s), riot path: {}",
                report.accounts_added, report.riot_path_imported
            ));
            Ok(Some(report))
        }
        Err(e) => {
            log_line(&format!("import from v3 failed: {e}"));
            Err(e)
        }
    }
}

fn convert(old: Vec<OldAccount>) -> Vec<Account> {
    old.into_iter()
        .filter(|o| !o.username.trim().is_empty())
        .map(|o| {
            let note = o.note.trim().to_string();
            let region = o.region.trim();
            let note = match (note.is_empty(), region.is_empty()) {
                (_, true) => note,
                (true, false) => format!("Server: {region}"),
                (false, false) => format!("{note} · Server: {region}"),
            };
            Account {
                id: new_id(),
                launcher: Launcher::Riot,
                label: String::new(),
                username: o.username.trim().to_string(),
                password: o.password,
                note,
                default_game: None,
                last_used: None,
            }
        })
        .collect()
}

fn read_old_accounts(path: &Path) -> Result<Vec<OldAccount>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("Không đọc được dữ liệu cũ: {e}"))?;
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let json = if let Some(payload) = raw.strip_prefix(RAM2_PREFIX) {
        let blob = B64.decode(payload.as_bytes()).map_err(|e| format!("Dữ liệu cũ hỏng: {e}"))?;
        let plain = crate::crypto::dpapi_unprotect(&blob)?;
        String::from_utf8(plain).map_err(|e| format!("Dữ liệu cũ hỏng: {e}"))?
    } else {
        java_decrypt(raw, &machine_guid()?)?
    };
    serde_json::from_str(&json).map_err(|e| format!("Dữ liệu cũ không đúng định dạng: {e}"))
}

fn apply_old_config(path: &Path, settings: &mut Settings) -> bool {
    let Some(obj) = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    else {
        return false;
    };
    if let Some(v) = obj.get("runAtStartup").and_then(|v| v.as_bool()) {
        settings.run_at_startup = v;
    }
    if let Some(v) = obj.get("autoClickLogin").and_then(|v| v.as_bool()) {
        settings.auto_submit = v;
    }
    match obj.get("riotClientPath").and_then(|v| v.as_str()) {
        Some(p) if settings.riot.client_path.is_empty() && Path::new(p).is_file() => {
            settings.riot.client_path = p.to_string();
            true
        }
        _ => false,
    }
}

/// Java format: Base64(AES-256/ECB/PKCS7(json)), key = SHA-256(machine id).
fn java_decrypt(b64: &str, machine_id: &str) -> Result<String, String> {
    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyInit};
    use sha2::{Digest, Sha256};
    type Aes256EcbDec = ecb::Decryptor<aes::Aes256>;

    let key: [u8; 32] = Sha256::digest(machine_id.as_bytes()).into();
    let mut buf = B64
        .decode(b64.trim().as_bytes())
        .map_err(|e| format!("Dữ liệu cũ hỏng: {e}"))?;
    let plain = Aes256EcbDec::new(&key.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| "Không giải mã được dữ liệu cũ (tạo trên máy khác?)".to_string())?;
    String::from_utf8(plain.to_vec()).map_err(|e| format!("Dữ liệu cũ hỏng: {e}"))
}

/// The Windows MachineGuid, as the Java app read it.
fn machine_guid() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = std::process::Command::new("reg")
            .args(["query", r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Không đọc được MachineGuid: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout);
        return text
            .lines()
            .find(|l| l.contains("MachineGuid"))
            .and_then(|l| l.split_whitespace().last())
            .map(str::to_string)
            .ok_or_else(|| "Không đọc được MachineGuid".to_string());
    }
    #[cfg(not(windows))]
    {
        return Err("Dữ liệu định dạng Java chỉ đọc được trên Windows".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test vector produced by the original Java CryptoService (from the v3 repo).
    const MACHINE_ID: &str = "TEST-MACHINE-GUID-1234";
    const PLAINTEXT: &str =
        "[{\"username\":\"u@example.com\",\"password\":\"p@ss_w0rd-1\",\"region\":\"VN\",\"note\":\"main\"}]";
    const CIPHER_B64: &str = "MRIryDr0wxwARmbPntThUZI8YfKYLdjEBf2vEZcENUVnmes37AyQE4dv6xOGbhK5Z4CQOBzW5TwayetBaI/7tSGPW96bfVXEZcXR1NiloS/UH5eqrKQyqiKUfKZhpvRj";

    #[test]
    fn decrypts_java_data() {
        assert_eq!(java_decrypt(CIPHER_B64, MACHINE_ID).unwrap(), PLAINTEXT);
        assert!(java_decrypt(CIPHER_B64, "other-machine").is_err());
    }

    #[test]
    fn converts_region_into_note() {
        let old: Vec<OldAccount> = serde_json::from_str(PLAINTEXT).unwrap();
        let new = convert(old);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].launcher, Launcher::Riot);
        assert_eq!(new[0].username, "u@example.com");
        assert_eq!(new[0].password, "p@ss_w0rd-1");
        assert_eq!(new[0].note, "main · Server: VN");
    }

    #[test]
    fn config_import() {
        let dir = std::env::temp_dir().join(format!("gas-legacy-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("RiotClientServices.exe");
        std::fs::write(&exe, b"x").unwrap();
        let cfg = dir.join("config.json");
        let json = serde_json::json!({
            "riotClientPath": exe.to_string_lossy(),
            "runAtStartup": true,
            "autoClickLogin": false
        });
        std::fs::write(&cfg, json.to_string()).unwrap();
        let mut s = Settings::default();
        assert!(apply_old_config(&cfg, &mut s));
        assert!(s.run_at_startup);
        assert!(!s.auto_submit);
        assert_eq!(s.riot.client_path, exe.to_string_lossy());
        let _ = std::fs::remove_dir_all(dir);
    }
}
