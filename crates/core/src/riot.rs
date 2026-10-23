//! Riot Client integration.
//!
//! * Sign-out = close every Riot process and delete
//!   `RiotGamesPrivateSettings.yaml` (the "Stay signed in" session), so the
//!   next start shows the sign-in page.
//! * Starting a game = `RiotClientServices.exe --launch-product=<id>
//!   --launch-patchline=live`, the same command the desktop shortcuts use. The
//!   client signs in first, then opens that game by itself.

use crate::model::Game;
use crate::os::{self, WindowInfo};
use crate::paths::{expand_env, log_line};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Riot Client processes (lower-case).
pub const CLIENT_PROCESSES: &[&str] = &[
    "riotclientservices.exe",
    "riot client.exe",
    "riotclientux.exe",
    "riotclientuxrender.exe",
    "riotclientcrashhandler.exe",
];

/// Game processes that are closed together with the client on sign-out.
pub const GAME_PROCESSES: &[&str] = &[
    "leagueclient.exe",
    "leagueclientux.exe",
    "leagueclientuxrender.exe",
    "league of legends.exe",
    "valorant.exe",
    "valorant-win64-shipping.exe",
];

const DEFAULT_PATH: &str = r"C:\Riot Games\Riot Client\RiotClientServices.exe";
const INSTALLS_JSON: &str = r"C:\ProgramData\Riot Games\RiotClientInstalls.json";
const SESSION_FILE: &str =
    r"%LOCALAPPDATA%\Riot Games\Riot Client\Data\RiotGamesPrivateSettings.yaml";

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Finds RiotClientServices.exe: Riot's own install manifest first, then the
/// default location.
pub fn detect_client_path() -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(INSTALLS_JSON) {
        if let Some(p) = parse_installs_json(&text) {
            if Path::new(&p).is_file() {
                return Some(p);
            }
        }
    }
    Path::new(DEFAULT_PATH).is_file().then(|| DEFAULT_PATH.to_string())
}

fn parse_installs_json(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    ["rc_default", "rc_live", "rc_beta"]
        .iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .map(|s| s.replace('/', "\\"))
}

/// Configured path, or auto-detected.
pub fn resolve_client_path(configured: &str) -> Result<PathBuf, String> {
    let configured = configured.trim();
    if !configured.is_empty() {
        let p = PathBuf::from(configured);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("Không tìm thấy Riot Client tại: {configured}"))
        };
    }
    detect_client_path()
        .map(PathBuf::from)
        .ok_or_else(|| "Không tìm thấy Riot Client. Hãy chọn RiotClientServices.exe trong Cài đặt.".into())
}

pub fn client_running() -> bool {
    os::any_running(&names(CLIENT_PROCESSES))
}

pub fn game_running() -> bool {
    os::any_running(&names(GAME_PROCESSES))
}

/// The main Riot Client window (largest window titled "Riot Client" or owned
/// by a Riot Client process).
pub fn find_window() -> Option<WindowInfo> {
    let client_pids: Vec<u32> = os::processes()
        .into_iter()
        .filter(|p| CLIENT_PROCESSES.contains(&p.name.as_str()))
        .map(|p| p.pid)
        .collect();
    os::find_window(|w| {
        let title = w.title.to_lowercase();
        title.contains("riot client") || (client_pids.contains(&w.pid) && !title.is_empty())
    })
}

/// Closes Riot Client and games, then removes the remembered session.
pub fn sign_out() -> Result<(), String> {
    let mut all = names(GAME_PROCESSES);
    all.extend(names(CLIENT_PROCESSES));
    let killed = os::kill_by_names(&all, Duration::from_secs(10));
    log_line(&format!("riot: closed {killed} process(es)"));
    if os::any_running(&all) {
        return Err("Không tắt được Riot Client (thử chạy app bằng quyền Administrator?)".into());
    }
    // The client may still flush its files right after exit.
    std::thread::sleep(Duration::from_millis(600));
    clear_session()
}

pub fn clear_session() -> Result<(), String> {
    let path = PathBuf::from(expand_env(SESSION_FILE));
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Không xoá được phiên đăng nhập cũ ({}): {e}", path.display()))?;
        log_line("riot: session file removed");
    }
    Ok(())
}

pub fn launch_args(game: Option<Game>) -> Vec<String> {
    match game.and_then(Game::riot_product) {
        Some(product) => vec![
            format!("--launch-product={product}"),
            "--launch-patchline=live".to_string(),
        ],
        None => Vec::new(),
    }
}

pub fn launch(client: &Path, game: Option<Game>) -> Result<(), String> {
    let args = launch_args(game);
    log_line(&format!("riot: launch {} {:?}", client.display(), args));
    os::spawn(client, &args)
}

/// Process that proves the requested game started.
pub fn game_started(game: Game) -> bool {
    let wanted: &[&str] = match game {
        Game::League => &["leagueclient.exe", "leagueclientux.exe", "league of legends.exe"],
        Game::Valorant => &["valorant.exe", "valorant-win64-shipping.exe"],
        Game::DeltaForce => &[],
    };
    os::any_running(&names(wanted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_json_prefers_default() {
        let json = r#"{"associated_client":{},"patchlines":{},"rc_default":"C:/Riot Games/Riot Client/RiotClientServices.exe","rc_live":"D:/x.exe"}"#;
        assert_eq!(
            parse_installs_json(json).as_deref(),
            Some(r"C:\Riot Games\Riot Client\RiotClientServices.exe")
        );
        assert_eq!(parse_installs_json(r#"{"rc_live":"D:/x.exe"}"#).as_deref(), Some(r"D:\x.exe"));
        assert_eq!(parse_installs_json("{}"), None);
    }

    #[test]
    fn args_per_game() {
        assert_eq!(
            launch_args(Some(Game::Valorant)),
            vec!["--launch-product=valorant", "--launch-patchline=live"]
        );
        assert_eq!(launch_args(Some(Game::League))[0], "--launch-product=league_of_legends");
        assert!(launch_args(None).is_empty());
        assert!(launch_args(Some(Game::DeltaForce)).is_empty());
    }
}
