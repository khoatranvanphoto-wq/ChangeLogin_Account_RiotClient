//! Global hotkeys:
//! * Ctrl+Alt+A — show / hide the window
//! * Ctrl+Alt+1 … Ctrl+Alt+9 — switch to account #1 … #9 (Riot accounts first,
//!   then Garena, in list order — the same numbers the UI shows).

use crate::AppState;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

const TOGGLE: &str = "ctrl+alt+a";
const DIGITS: [Code; 9] = [
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];

pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let mods = Modifiers::CONTROL | Modifiers::ALT;
            if shortcut.matches(mods, Code::KeyA) {
                crate::toggle_main_window(app);
                return;
            }
            if let Some(n) = DIGITS.iter().position(|c| shortcut.matches(mods, *c)) {
                let id: Option<String> = {
                    let state = app.state::<AppState>();
                    let accounts = state.accounts.lock().unwrap();
                    let order = crate::commands::hotkey_order(&accounts);
                    let picked = order.get(n).map(|a| a.id.clone());
                    picked
                };
                if let Some(id) = id {
                    crate::request_switch(app, id, "hotkey");
                }
            }
        })
        .build()
}

/// (Re)registers the hotkeys. A combination already taken by another app is
/// skipped and logged instead of failing startup.
pub fn apply(app: &AppHandle, enabled: bool) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if !enabled {
        return;
    }
    let mut all = vec![TOGGLE.to_string()];
    all.extend((1..=9).map(|n| format!("ctrl+alt+{n}")));
    for combo in all {
        if let Err(e) = gs.register(combo.as_str()) {
            gas_core::paths::log_line(&format!("hotkey {combo} not registered: {e}"));
        }
    }
}
