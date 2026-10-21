//! Interaction with the external Riot Client: process/window detection, focusing,
//! launching, and automated credential entry.
//!
//! Login locates the username/password fields through Windows UI Automation (the
//! live accessibility tree). That is independent of window size, DPI, and Riot
//! Client layout changes. Keyboard-only fill is the fallback. No layout-ratio
//! mouse clicks. Only standard OS APIs and input simulation are used
//! (no hooking/injection) — Vanguard-safe.

use crate::model::{Account, RiotStatus};

pub fn status() -> RiotStatus {
    RiotStatus {
        running: is_riot_client_running(),
        window_visible: is_riot_client_window_visible(),
    }
}

pub fn is_riot_client_running() -> bool {
    #[cfg(windows)]
    {
        return crate::riot_windows::is_riot_client_running();
    }
    #[cfg(not(windows))]
    false
}

pub fn is_riot_client_window_visible() -> bool {
    #[cfg(windows)]
    {
        return crate::riot_windows::is_riot_client_window_visible();
    }
    #[cfg(not(windows))]
    false
}

pub fn launch_riot_client(path: &str) -> bool {
    if !std::path::Path::new(path).exists() {
        return false;
    }
    std::process::Command::new(path).spawn().is_ok()
}

pub fn focus_riot_client_window() -> bool {
    #[cfg(windows)]
    {
        return crate::riot_windows::focus_riot_client_window();
    }
    #[cfg(not(windows))]
    false
}

/// Fills the Riot Client login form for the given account, optionally submitting (Enter).
pub fn auto_login(account: &Account, auto_submit: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        return crate::riot_windows::auto_login(account, auto_submit);
    }
    #[cfg(not(windows))]
    {
        let _ = (account, auto_submit);
        Err("Auto login is only supported on Windows".into())
    }
}
