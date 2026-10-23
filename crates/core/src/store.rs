//! Encrypted account storage with atomic writes.
//!
//! Save path: JSON → protect → `accounts.tmp` → copy current file to
//! `accounts.bak` → rename tmp over the real file. A failed load never
//! touches the file on disk.

use crate::crypto;
use crate::model::{new_id, Account, AccountInput, AccountView};
use crate::paths;
use std::fs;
use std::path::Path;

pub fn load() -> Result<Vec<Account>, String> {
    load_from(&paths::accounts_file())
}

pub fn load_from(path: &Path) -> Result<Vec<Account>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("Không đọc được {path:?}: {e}"))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let json = crypto::unprotect(&raw)?;
    serde_json::from_str(&json).map_err(|e| format!("File tài khoản bị hỏng: {e}"))
}

pub fn save(accounts: &[Account]) -> Result<(), String> {
    paths::ensure_data_dir().map_err(|e| e.to_string())?;
    save_to(&paths::accounts_file(), accounts)
}

pub fn save_to(path: &Path, accounts: &[Account]) -> Result<(), String> {
    let json = serde_json::to_string(accounts).map_err(|e| e.to_string())?;
    let sealed = crypto::protect(&json)?;
    write_atomic(path, sealed.as_bytes(), true)
}

/// Writes `bytes` to `path` via a temp file + rename. With `keep_backup`, the
/// previous version is copied to `<path>.bak` first.
pub fn write_atomic(path: &Path, bytes: &[u8], keep_backup: bool) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("Ghi file tạm lỗi: {e}"))?;
    if keep_backup && path.exists() {
        let bak = path.with_extension("bak");
        let _ = fs::copy(path, bak);
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Không thay được file dữ liệu: {e}")
    })
}

pub fn views(accounts: &[Account]) -> Vec<AccountView> {
    accounts.iter().map(AccountView::from).collect()
}

/// Applies an add/edit to `accounts` and returns the resulting account id.
pub fn upsert(accounts: &mut Vec<Account>, input: AccountInput) -> Result<String, String> {
    input.validate()?;
    let username = input.username.trim().to_string();

    let duplicate = accounts.iter().any(|a| {
        a.launcher == input.launcher
            && a.username.eq_ignore_ascii_case(&username)
            && Some(&a.id) != input.id.as_ref()
    });
    if duplicate {
        return Err(format!("Tài khoản \"{username}\" đã có trong danh sách"));
    }

    match input.id {
        Some(id) => {
            let acc = accounts
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or("Không tìm thấy tài khoản cần sửa")?;
            acc.launcher = input.launcher;
            acc.label = input.label.trim().to_string();
            acc.username = username;
            if !input.password.is_empty() {
                acc.password = input.password;
            }
            acc.note = input.note.trim().to_string();
            acc.default_game = input.default_game;
            Ok(id)
        }
        None => {
            let id = new_id();
            accounts.push(Account {
                id: id.clone(),
                launcher: input.launcher,
                label: input.label.trim().to_string(),
                username,
                password: input.password,
                note: input.note.trim().to_string(),
                default_game: input.default_game,
                last_used: None,
            });
            Ok(id)
        }
    }
}

/// Moves account `id` one step within its own launcher group (`delta` = -1 / +1).
pub fn move_within_launcher(accounts: &mut [Account], id: &str, delta: i32) -> bool {
    let Some(pos) = accounts.iter().position(|a| a.id == id) else {
        return false;
    };
    let launcher = accounts[pos].launcher;
    let same: Vec<usize> = accounts
        .iter()
        .enumerate()
        .filter(|(_, a)| a.launcher == launcher)
        .map(|(i, _)| i)
        .collect();
    let Some(k) = same.iter().position(|&i| i == pos) else {
        return false;
    };
    let target = k as i32 + delta;
    if target < 0 || target as usize >= same.len() {
        return false;
    }
    accounts.swap(pos, same[target as usize]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Game, Launcher};

    fn input(user: &str) -> AccountInput {
        AccountInput {
            id: None,
            launcher: Launcher::Riot,
            label: "Main".into(),
            username: user.into(),
            password: "secret".into(),
            note: String::new(),
            default_game: Some(Game::Valorant),
        }
    }

    #[test]
    fn add_edit_keeps_password_when_blank() {
        let mut list = Vec::new();
        let id = upsert(&mut list, input("alpha")).unwrap();
        let mut edit = input("alpha2");
        edit.id = Some(id.clone());
        edit.password = String::new();
        upsert(&mut list, edit).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].username, "alpha2");
        assert_eq!(list[0].password, "secret");
    }

    #[test]
    fn rejects_duplicates_per_launcher() {
        let mut list = Vec::new();
        upsert(&mut list, input("alpha")).unwrap();
        assert!(upsert(&mut list, input("ALPHA")).is_err());
        let mut garena = input("alpha");
        garena.launcher = Launcher::Garena;
        garena.default_game = None;
        assert!(upsert(&mut list, garena).is_ok());
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("gas-test-{}", new_id()));
        let path = dir.join("accounts.dat");
        let mut list = Vec::new();
        upsert(&mut list, input("alpha")).unwrap();
        save_to(&path, &list).unwrap();
        save_to(&path, &list).unwrap(); // second save creates .bak
        assert!(path.with_extension("bak").exists());
        assert_eq!(load_from(&path).unwrap(), list);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn move_stays_within_group() {
        let mut list = Vec::new();
        upsert(&mut list, input("a")).unwrap();
        let mut g = input("g");
        g.launcher = Launcher::Garena;
        g.default_game = None;
        upsert(&mut list, g).unwrap();
        upsert(&mut list, input("b")).unwrap();
        let b = list[2].id.clone();
        assert!(move_within_launcher(&mut list, &b, -1));
        assert_eq!(list[0].username, "b");
        assert_eq!(list[1].username, "g");
        assert!(!move_within_launcher(&mut list, &b, -1));
    }
}
