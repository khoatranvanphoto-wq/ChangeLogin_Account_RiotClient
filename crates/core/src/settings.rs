//! User settings (`settings.json`). Contains no secrets.

use crate::geometry::RelPoint;
use crate::paths;
use crate::store::write_atomic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Press Enter after typing the password.
    pub auto_submit: bool,
    /// Ask which Riot game to start every time (otherwise use the account's default).
    pub ask_game_each_time: bool,
    /// Closing the window hides it to the tray.
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub run_at_startup: bool,
    /// Ctrl+Alt+A (show/hide) and Ctrl+Alt+1..9 (switch to account N).
    pub hotkeys_enabled: bool,
    pub riot: RiotSettings,
    pub garena: GarenaSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_submit: true,
            ask_game_each_time: true,
            close_to_tray: true,
            start_minimized: false,
            run_at_startup: false,
            hotkeys_enabled: true,
            riot: RiotSettings::default(),
            garena: GarenaSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RiotSettings {
    /// Path to RiotClientServices.exe. Empty = auto-detect.
    pub client_path: String,
    /// Optional calibrated position of the username field.
    pub username_point: Option<RelPoint>,
    /// Milliseconds to wait after the window appears before typing.
    pub ready_delay_ms: u64,
}

impl Default for RiotSettings {
    fn default() -> Self {
        RiotSettings { client_path: String::new(), username_point: None, ready_delay_ms: 2500 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GarenaSettings {
    /// Path to the Garena / Delta Force launcher executable.
    pub launcher_path: String,
    /// Click this first (e.g. the "Garena account" tab) before typing.
    pub login_tab_point: Option<RelPoint>,
    /// Calibrated username field.
    pub username_point: Option<RelPoint>,
    /// Calibrated "Start game" button, clicked after login for Delta Force.
    pub launch_button_point: Option<RelPoint>,
    /// Milliseconds to wait after the launcher window appears before typing.
    pub ready_delay_ms: u64,
    /// Milliseconds to wait after submitting login before clicking "Start game".
    pub launch_delay_ms: u64,
    /// Files or folders that hold the remembered login; deleted on sign-out.
    /// `%LOCALAPPDATA%`-style variables are expanded.
    pub session_paths: Vec<String>,
    /// Game executables (e.g. the Delta Force client) closed on sign-out and
    /// used to tell whether the game is running.
    pub game_processes: Vec<String>,
}

impl Default for GarenaSettings {
    fn default() -> Self {
        GarenaSettings {
            launcher_path: String::new(),
            login_tab_point: None,
            username_point: None,
            launch_button_point: None,
            ready_delay_ms: 3000,
            launch_delay_ms: 10000,
            session_paths: Vec::new(),
            game_processes: Vec::new(),
        }
    }
}

impl Settings {
    /// Clamp values the UI could send out of range.
    pub fn sanitized(mut self) -> Self {
        self.riot.ready_delay_ms = self.riot.ready_delay_ms.clamp(0, 30_000);
        self.garena.ready_delay_ms = self.garena.ready_delay_ms.clamp(0, 30_000);
        self.garena.launch_delay_ms = self.garena.launch_delay_ms.clamp(0, 120_000);
        self.riot.client_path = self.riot.client_path.trim().to_string();
        self.garena.launcher_path = self.garena.launcher_path.trim().to_string();
        let clean = |v: Vec<String>| -> Vec<String> {
            v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        };
        self.garena.session_paths = clean(self.garena.session_paths);
        self.garena.game_processes = clean(self.garena.game_processes);
        self
    }
}

pub fn load() -> Settings {
    let path = paths::settings_file();
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Settings>(&text)
            .map(Settings::sanitized)
            .unwrap_or_else(|e| {
                log::warn!("settings.json không hợp lệ, dùng mặc định: {e}");
                Settings::default()
            }),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    write_atomic(&paths::settings_file(), json.as_bytes(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let s: Settings = serde_json::from_str(r#"{"auto_submit":false,"garena":{"launch_delay_ms":5}}"#).unwrap();
        assert!(!s.auto_submit);
        assert!(s.close_to_tray);
        assert_eq!(s.garena.launch_delay_ms, 5);
        assert_eq!(s.garena.ready_delay_ms, 3000);
        assert_eq!(s.riot.ready_delay_ms, 2500);
    }

    #[test]
    fn sanitize_trims_lists() {
        let mut s = Settings::default();
        s.garena.session_paths = vec!["  ".into(), " %LOCALAPPDATA%\\x ".into()];
        s.garena.launch_delay_ms = 10_000_000;
        let s = s.sanitized();
        assert_eq!(s.garena.session_paths, vec!["%LOCALAPPDATA%\\x".to_string()]);
        assert_eq!(s.garena.launch_delay_ms, 120_000);
    }
}
