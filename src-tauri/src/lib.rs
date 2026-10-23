mod commands;
mod hotkeys;
mod tray;

use gas_core::model::Account;
use gas_core::state::ActiveState;
use gas_core::Settings;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{Emitter, Manager, WindowEvent};

/// Shared state behind every command.
pub struct AppState {
    pub accounts: Mutex<Vec<Account>>,
    pub settings: Mutex<Settings>,
    pub active: Mutex<ActiveState>,
    /// Set when `accounts.dat` exists but could not be read. While set, the
    /// app refuses to write accounts so the original file is never overwritten.
    pub load_error: Mutex<Option<String>>,
    /// A switch / sign-out is in progress (blocks hotkey spam).
    pub busy: AtomicBool,
    /// Result of importing data from the old v3 app, shown once by the UI.
    pub import_notice: Mutex<Option<ImportNotice>>,
}

#[derive(Clone, serde::Serialize)]
pub struct ImportNotice {
    pub report: Option<gas_core::legacy::ImportReport>,
    pub error: Option<String>,
}

impl AppState {
    fn load() -> Self {
        let (accounts, load_error) = match gas_core::store::load() {
            Ok(list) => (list, None),
            Err(e) => {
                gas_core::paths::log_line(&format!("load accounts failed: {e}"));
                (Vec::new(), Some(e))
            }
        };
        let mut accounts = accounts;
        let mut settings = gas_core::settings::load();
        let mut import_notice = None;
        // Never import on top of a file we failed to read: we couldn't save anyway.
        if load_error.is_none() {
            match gas_core::legacy::import_once(&mut accounts, &mut settings) {
                Ok(Some(report)) => {
                    if report.accounts_added > 0 {
                        if let Err(e) = gas_core::store::save(&accounts) {
                            gas_core::paths::log_line(&format!("save imported accounts failed: {e}"));
                        }
                    }
                    let _ = gas_core::settings::save(&settings);
                    import_notice = Some(ImportNotice { report: Some(report), error: None });
                }
                Ok(None) => {}
                Err(e) => import_notice = Some(ImportNotice { report: None, error: Some(e) }),
            }
        }
        AppState {
            accounts: Mutex::new(accounts),
            settings: Mutex::new(settings),
            active: Mutex::new(gas_core::state::load()),
            load_error: Mutex::new(load_error),
            busy: AtomicBool::new(false),
            import_notice: Mutex::new(import_notice),
        }
    }
}

/// Payload of the `request-switch` event: the tray or a hotkey asks the UI to
/// switch, so the UI can show its game picker and progress.
#[derive(Clone, serde::Serialize)]
pub struct SwitchRequest {
    pub id: String,
    pub source: &'static str,
}

pub fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let visible = w.is_visible().unwrap_or(false);
        let minimized = w.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = w.hide();
        } else {
            show_main_window(app);
        }
    }
}

pub fn request_switch(app: &tauri::AppHandle, id: String, source: &'static str) {
    show_main_window(app);
    let _ = app.emit("request-switch", SwitchRequest { id, source });
}

pub fn request_sign_out(app: &tauri::AppHandle, launcher: gas_core::Launcher) {
    show_main_window(app);
    let _ = app.emit("request-sign-out", launcher);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = gas_core::paths::ensure_data_dir();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(hotkeys::plugin())
        .manage(AppState::load())
        .setup(|app| {
            let handle = app.handle().clone();
            tray::create(&handle)?;

            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            hotkeys::apply(&handle, settings.hotkeys_enabled);
            commands::sync_autostart(&handle, settings.run_at_startup);

            let from_autostart = std::env::args().any(|a| a == "--minimized");
            if !(from_autostart || settings.start_minimized) {
                show_main_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let to_tray = window
                    .state::<AppState>()
                    .settings
                    .lock()
                    .map(|s| s.close_to_tray)
                    .unwrap_or(true);
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::save_account,
            commands::delete_account,
            commands::move_account,
            commands::set_default_game,
            commands::save_settings,
            commands::detect_riot_path,
            commands::status,
            commands::switch_account,
            commands::sign_out,
            commands::capture_point,
            commands::export_backup,
            commands::import_backup,
            commands::open_data_dir,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Game Account Switcher");
}
