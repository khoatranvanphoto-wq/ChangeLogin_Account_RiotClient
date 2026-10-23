//! Garena launcher (Delta Force) integration.
//!
//! Garena doesn't document its launcher, so nothing here is hard-coded beyond
//! "the executable you pick in Settings":
//!
//! * The launcher's processes are every process whose exe lives in the same
//!   folder tree as the configured launcher.
//! * Sign-out closes those (plus any configured game processes) and deletes
//!   the session files/folders listed in Settings, if any.
//! * Fields and the "Start game" button can be calibrated as points relative to
//!   the launcher window; without calibration we fall back to UI Automation and
//!   then to probing the middle of the window for a text field.

use crate::os::{self, WindowInfo};
use crate::paths::{expand_env, log_line};
use crate::settings::GarenaSettings;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn launcher_path(s: &GarenaSettings) -> Result<PathBuf, String> {
    let raw = s.launcher_path.trim();
    if raw.is_empty() {
        return Err("Chưa chọn file launcher Garena / Delta Force trong Cài đặt.".into());
    }
    let p = PathBuf::from(raw);
    if p.is_file() {
        Ok(p)
    } else {
        Err(format!("Không tìm thấy launcher tại: {raw}"))
    }
}

fn install_dir(launcher: &Path) -> Option<&Path> {
    launcher.parent()
}

fn launcher_pids(s: &GarenaSettings) -> Vec<u32> {
    match launcher_path(s) {
        Ok(p) => install_dir(&p).map(os::pids_in_dir).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn launcher_running(s: &GarenaSettings) -> bool {
    !launcher_pids(s).is_empty()
}

pub fn game_running(s: &GarenaSettings) -> bool {
    !s.game_processes.is_empty() && os::any_running(&s.game_processes)
}

/// The launcher's biggest window — the sign-in dialog before login, the main
/// window after.
pub fn find_window(s: &GarenaSettings) -> Option<WindowInfo> {
    let pids = launcher_pids(s);
    if pids.is_empty() {
        return None;
    }
    os::find_window(|w| pids.contains(&w.pid))
}

pub fn sign_out(s: &GarenaSettings) -> Result<(), String> {
    let mut killed = 0;
    if !s.game_processes.is_empty() {
        killed += os::kill_by_names(&s.game_processes, Duration::from_secs(8));
    }
    // A launcher can respawn helpers while closing; sweep a few times.
    for _ in 0..4 {
        let pids = launcher_pids(s);
        if pids.is_empty() {
            break;
        }
        killed += os::kill_pids(&pids);
        std::thread::sleep(Duration::from_millis(700));
    }
    log_line(&format!("garena: closed {killed} process(es)"));
    if launcher_running(s) {
        return Err("Không tắt được launcher Garena (thử chạy app bằng quyền Administrator?)".into());
    }
    std::thread::sleep(Duration::from_millis(500));
    clear_session(s)
}

pub fn clear_session(s: &GarenaSettings) -> Result<(), String> {
    for raw in &s.session_paths {
        let path = PathBuf::from(expand_env(raw));
        if !is_safe_to_delete(&path) {
            return Err(format!("Bỏ qua đường dẫn không an toàn: {}", path.display()));
        }
        let res = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else if path.exists() {
            std::fs::remove_file(&path)
        } else {
            Ok(())
        };
        res.map_err(|e| format!("Không xoá được {}: {e}", path.display()))?;
        log_line(&format!("garena: cleared {}", path.display()));
    }
    Ok(())
}

/// Refuses to delete drive roots, user-profile roots and anything with fewer
/// than three path components — a typo in Settings must not wipe a disk.
fn is_safe_to_delete(path: &Path) -> bool {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return false;
    }
    if path.to_string_lossy().contains('%') {
        return false; // unexpanded variable
    }
    let depth = path
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count();
    if depth < 3 {
        return false;
    }
    let lowered = path.to_string_lossy().to_lowercase().replace('/', "\\");
    let forbidden = ["\\windows", "\\program files", "\\programdata\\microsoft"];
    !forbidden.iter().any(|f| lowered.ends_with(f))
}

pub fn launch(s: &GarenaSettings) -> Result<(), String> {
    let p = launcher_path(s)?;
    log_line(&format!("garena: launch {}", p.display()));
    os::spawn(&p, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn safety_rules() {
        assert!(!is_safe_to_delete(Path::new(r"C:\")));
        assert!(!is_safe_to_delete(Path::new(r"C:\Users")));
        assert!(!is_safe_to_delete(Path::new(r"C:\Users\me")));
        assert!(!is_safe_to_delete(Path::new(r"relative\path\x\y")));
        assert!(is_safe_to_delete(Path::new(r"C:\Users\me\AppData\Local\Garena\session")));
    }

    #[cfg(not(windows))]
    #[test]
    fn safety_rules() {
        assert!(!is_safe_to_delete(Path::new("/")));
        assert!(!is_safe_to_delete(Path::new("/home/me")));
        assert!(!is_safe_to_delete(Path::new("rel/a/b/c")));
        assert!(is_safe_to_delete(Path::new("/home/me/.garena/session")));
    }

    #[test]
    fn missing_path_is_error() {
        let s = GarenaSettings::default();
        assert!(launcher_path(&s).is_err());
    }
}
