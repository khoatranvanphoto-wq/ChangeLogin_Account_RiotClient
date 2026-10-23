//! System tray: left click toggles the window; the menu lists every account
//! (grouped per launcher) for one-click switching.

use crate::AppState;
use gas_core::model::{Account, Launcher};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

const TRAY_ID: &str = "main-tray";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Game Account Switcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id.0.as_str()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::toggle_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Rebuilds the menu after accounts or the active account changed.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match build_menu(app) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(e) => gas_core::paths::log_line(&format!("tray menu rebuild failed: {e}")),
    }
}

fn label_for(a: &Account, active: bool, hotkey: Option<usize>) -> String {
    let name = if a.label.trim().is_empty() {
        a.username.clone()
    } else {
        format!("{} ({})", a.label, a.username)
    };
    let mark = if active { "● " } else { "" };
    match hotkey {
        Some(n) => format!("{mark}{name}\tCtrl+Alt+{n}"),
        None => format!("{mark}{name}"),
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let accounts = state.accounts.lock().unwrap().clone();
    let active = state.active.lock().unwrap().clone();
    let hotkeys_on = state.settings.lock().unwrap().hotkeys_enabled;
    let order: Vec<String> = crate::commands::hotkey_order(&accounts)
        .iter()
        .map(|a| a.id.clone())
        .collect();

    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "show", "Mở Game Account Switcher", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    for launcher in [Launcher::Riot, Launcher::Garena] {
        let group: Vec<&Account> = accounts.iter().filter(|a| a.launcher == launcher).collect();
        let title = match launcher {
            Launcher::Riot => "Riot Client",
            Launcher::Garena => "Garena · Delta Force",
        };
        let sub = Submenu::with_id(app, format!("group:{launcher:?}"), title, true)?;
        if group.is_empty() {
            sub.append(&MenuItem::with_id(
                app,
                format!("empty:{launcher:?}"),
                "(chưa có tài khoản)",
                false,
                None::<&str>,
            )?)?;
        }
        for a in group {
            let hotkey = order
                .iter()
                .position(|id| id == &a.id)
                .filter(|&i| hotkeys_on && i < 9)
                .map(|i| i + 1);
            let is_active = active.get(launcher) == Some(a.id.as_str());
            sub.append(&MenuItem::with_id(
                app,
                format!("acc:{}", a.id),
                label_for(a, is_active, hotkey),
                true,
                None::<&str>,
            )?)?;
        }
        sub.append(&PredefinedMenuItem::separator(app)?)?;
        let signout_id = match launcher {
            Launcher::Riot => "signout:riot",
            Launcher::Garena => "signout:garena",
        };
        sub.append(&MenuItem::with_id(app, signout_id, "Đăng xuất", true, None::<&str>)?)?;
        menu.append(&sub)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Thoát", true, None::<&str>)?)?;
    Ok(menu)
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => crate::show_main_window(app),
        "quit" => app.exit(0),
        "signout:riot" => crate::request_sign_out(app, Launcher::Riot),
        "signout:garena" => crate::request_sign_out(app, Launcher::Garena),
        other => {
            if let Some(acc) = other.strip_prefix("acc:") {
                crate::request_switch(app, acc.to_string(), "tray");
            }
        }
    }
}
