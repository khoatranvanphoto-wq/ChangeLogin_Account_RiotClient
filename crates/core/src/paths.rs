//! Where the app keeps its files: `%APPDATA%\GameAccountSwitcher` on Windows,
//! `~/.config/game-account-switcher` elsewhere (dev only).

use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GAS_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("GameAccountSwitcher");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("game-account-switcher")
}

pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn accounts_file() -> PathBuf {
    data_dir().join("accounts.dat")
}

pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn state_file() -> PathBuf {
    data_dir().join("state.json")
}

pub fn log_file() -> PathBuf {
    data_dir().join("switch.log")
}

/// Expands `%VAR%` segments (Windows style) using the current environment.
pub fn expand_env(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) if !name.is_empty() => out.push_str(&v),
                    _ => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Appends a line to the switch log (best effort, never fails the caller).
pub fn log_line(msg: &str) {
    log::info!("{msg}");
    if let Ok(dir) = ensure_data_dir() {
        use std::io::Write;
        let path = dir.join("switch.log");
        // Keep the log small: start over once it passes ~512 KB.
        if std::fs::metadata(&path).map(|m| m.len() > 512 * 1024).unwrap_or(false) {
            let _ = std::fs::remove_file(&path);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "[{}] {msg}", crate::model::now_unix());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_known_and_keeps_unknown() {
        std::env::set_var("GAS_TEST_VAR", "C:\\X");
        assert_eq!(expand_env("%GAS_TEST_VAR%\\a.yaml"), "C:\\X\\a.yaml");
        assert_eq!(expand_env("%NOPE_NOT_SET_123%\\b"), "%NOPE_NOT_SET_123%\\b");
        assert_eq!(expand_env("50% off"), "50% off");
        assert_eq!(expand_env("plain"), "plain");
    }
}
