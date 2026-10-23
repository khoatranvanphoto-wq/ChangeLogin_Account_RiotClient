//! Commands invoked from the React UI.
//!
//! Lock discipline: never call `tray::refresh` (which locks state itself) while
//! holding one of the state mutexes.

use crate::{hotkeys, tray, AppState};
use gas_core::backup;
use gas_core::geometry::RelPoint;
use gas_core::model::{now_unix, Account, AccountInput, AccountView, Game, Launcher};
use gas_core::state::ActiveState;
use gas_core::switcher::{self, LauncherStatus, Outcome, Progress};
use gas_core::{paths, store, Settings};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

/// Riot accounts first, then Garena, each in list order. Hotkey N = item N-1.
pub fn hotkey_order(accounts: &[Account]) -> Vec<&Account> {
    let mut out: Vec<&Account> = accounts.iter().filter(|a| a.launcher == Launcher::Riot).collect();
    out.extend(accounts.iter().filter(|a| a.launcher == Launcher::Garena));
    out
}

struct Busy<'a>(&'a AtomicBool);

impl<'a> Busy<'a> {
    fn take(flag: &'a AtomicBool) -> CmdResult<Self> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Đang xử lý một tài khoản khác, vui lòng đợi…".to_string())?;
        Ok(Busy(flag))
    }
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn ensure_writable(state: &AppState) -> CmdResult<()> {
    match state.load_error.lock().unwrap().as_ref() {
        Some(e) => Err(format!(
            "Không đọc được dữ liệu tài khoản ({e}). Để không ghi đè dữ liệu cũ, app tạm khoá việc lưu. \
             Bản sao lưu tự động nằm ở accounts.bak trong thư mục dữ liệu."
        )),
        None => Ok(()),
    }
}

/// Saves, returns fresh views, then refreshes the tray (after the lock is released).
fn commit(app: &AppHandle, f: impl FnOnce(&mut Vec<Account>) -> CmdResult<()>) -> CmdResult<Vec<AccountView>> {
    let state = app.state::<AppState>();
    ensure_writable(&state)?;
    let views = {
        let mut accounts = state.accounts.lock().unwrap();
        let mut next = accounts.clone();
        f(&mut next)?;
        store::save(&next)?;
        *accounts = next;
        store::views(&accounts)
    };
    tray::refresh(app);
    Ok(views)
}

fn spawn<T: Send + 'static>(f: impl FnOnce() -> CmdResult<T> + Send + 'static) -> impl std::future::Future<Output = CmdResult<T>> {
    async move {
        tauri::async_runtime::spawn_blocking(f)
            .await
            .map_err(|e| format!("Lỗi nội bộ: {e}"))?
    }
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Bootstrap {
    accounts: Vec<AccountView>,
    settings: Settings,
    active: ActiveState,
    load_error: Option<String>,
    supported: bool,
    version: String,
    data_dir: String,
    import_notice: Option<crate::ImportNotice>,
}

#[tauri::command]
pub fn bootstrap(app: AppHandle, state: State<'_, AppState>) -> Bootstrap {
    Bootstrap {
        accounts: store::views(&state.accounts.lock().unwrap()),
        settings: state.settings.lock().unwrap().clone(),
        active: state.active.lock().unwrap().clone(),
        load_error: state.load_error.lock().unwrap().clone(),
        supported: cfg!(windows),
        version: app.package_info().version.to_string(),
        data_dir: paths::data_dir().display().to_string(),
        import_notice: state.import_notice.lock().unwrap().take(),
    }
}

#[tauri::command]
pub fn save_account(app: AppHandle, input: AccountInput) -> CmdResult<Vec<AccountView>> {
    commit(&app, |list| store::upsert(list, input).map(|_| ()))
}

#[tauri::command]
pub fn delete_account(app: AppHandle, id: String) -> CmdResult<Vec<AccountView>> {
    let views = commit(&app, |list| {
        let before = list.len();
        list.retain(|a| a.id != id);
        if list.len() == before {
            return Err("Không tìm thấy tài khoản".into());
        }
        Ok(())
    })?;
    let state = app.state::<AppState>();
    let mut active = state.active.lock().unwrap();
    active.forget(&id);
    let _ = gas_core::state::save(&active);
    Ok(views)
}

#[tauri::command]
pub fn move_account(app: AppHandle, id: String, delta: i32) -> CmdResult<Vec<AccountView>> {
    commit(&app, |list| {
        store::move_within_launcher(list, &id, delta.signum());
        Ok(())
    })
}

#[tauri::command]
pub fn set_default_game(app: AppHandle, id: String, game: Option<Game>) -> CmdResult<Vec<AccountView>> {
    commit(&app, |list| {
        let acc = list.iter_mut().find(|a| a.id == id).ok_or("Không tìm thấy tài khoản")?;
        if let Some(g) = game {
            if g.launcher() != acc.launcher {
                return Err("Game không thuộc launcher của tài khoản này".into());
            }
        }
        acc.default_game = game;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let current = al.is_enabled().unwrap_or(false);
    let res = match (enabled, current) {
        (true, false) => al.enable(),
        (false, true) => al.disable(),
        _ => Ok(()),
    };
    if let Err(e) = res {
        paths::log_line(&format!("autostart: {e}"));
    }
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> CmdResult<Settings> {
    let settings = settings.sanitized();
    gas_core::settings::save(&settings)?;
    {
        let state = app.state::<AppState>();
        *state.settings.lock().unwrap() = settings.clone();
    }
    hotkeys::apply(&app, settings.hotkeys_enabled);
    sync_autostart(&app, settings.run_at_startup);
    tray::refresh(&app);
    Ok(settings)
}

#[tauri::command]
pub fn detect_riot_path() -> Option<String> {
    gas_core::riot::detect_client_path()
}

// ---------------------------------------------------------------------------
// Launchers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct StatusView {
    riot: LauncherStatus,
    garena: LauncherStatus,
    active: ActiveState,
    busy: bool,
}

#[tauri::command]
pub async fn status(app: AppHandle) -> CmdResult<StatusView> {
    spawn(move || {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap().clone();
        let active = state.active.lock().unwrap().clone();
        Ok(StatusView {
            riot: switcher::status(Launcher::Riot, &settings),
            garena: switcher::status(Launcher::Garena, &settings),
            active,
            busy: state.busy.load(Ordering::SeqCst),
        })
    })
    .await
}

#[tauri::command]
pub async fn switch_account(app: AppHandle, id: String, game: Option<Game>) -> CmdResult<Outcome> {
    spawn(move || run_switch(&app, &id, game)).await
}

fn run_switch(app: &AppHandle, id: &str, game: Option<Game>) -> CmdResult<Outcome> {
    let state = app.state::<AppState>();
    let _busy = Busy::take(&state.busy)?;

    let account = state
        .accounts
        .lock()
        .unwrap()
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or("Không tìm thấy tài khoản")?;
    let settings = state.settings.lock().unwrap().clone();
    let active_id = state.active.lock().unwrap().get(account.launcher).map(str::to_string);

    let report = |p: Progress| {
        let _ = app.emit("switch-progress", p);
    };
    let result = switcher::switch(&account, game, &settings, active_id.as_deref(), &report);

    {
        let mut active = state.active.lock().unwrap();
        // On failure we may already have signed the old account out, so the
        // launcher's state is unknown.
        let now = if result.is_ok() { Some(account.id.clone()) } else { None };
        active.set(account.launcher, now);
        let _ = gas_core::state::save(&active);
    }
    if result.is_ok() && ensure_writable(&state).is_ok() {
        let mut accounts = state.accounts.lock().unwrap();
        if let Some(a) = accounts.iter_mut().find(|a| a.id == account.id) {
            a.last_used = Some(now_unix());
        }
        let _ = store::save(&accounts);
    }
    tray::refresh(app);
    let _ = app.emit("accounts-changed", ());
    result
}

#[tauri::command]
pub async fn sign_out(app: AppHandle, launcher: Launcher) -> CmdResult<()> {
    spawn(move || {
        let state = app.state::<AppState>();
        let _busy = Busy::take(&state.busy)?;
        let settings = state.settings.lock().unwrap().clone();
        let res = switcher::sign_out(launcher, &settings);
        {
            let mut active = state.active.lock().unwrap();
            active.set(launcher, None);
            let _ = gas_core::state::save(&active);
        }
        tray::refresh(&app);
        res
    })
    .await
}

/// Hides this window, waits `delay_ms` for the user to hover the target in the
/// launcher, then records the mouse position relative to the launcher window.
#[tauri::command]
pub async fn capture_point(app: AppHandle, launcher: Launcher, delay_ms: u64) -> CmdResult<RelPoint> {
    spawn(move || {
        let settings = app.state::<AppState>().settings.lock().unwrap().clone();
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.minimize();
        }
        std::thread::sleep(Duration::from_millis(delay_ms.clamp(1000, 15_000)));
        let res = switcher::capture_point(launcher, &settings);
        crate::show_main_window(&app);
        res
    })
    .await
}

// ---------------------------------------------------------------------------
// Backup
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn export_backup(state: State<'_, AppState>, path: String, password: String) -> CmdResult<usize> {
    let accounts = state.accounts.lock().unwrap().clone();
    let text = backup::export(&accounts, &password)?;
    store::write_atomic(std::path::Path::new(&path), text.as_bytes(), false)?;
    Ok(accounts.len())
}

#[derive(Serialize)]
pub struct ImportResult {
    added: usize,
    accounts: Vec<AccountView>,
}

#[tauri::command]
pub fn import_backup(app: AppHandle, path: String, password: String, replace: bool) -> CmdResult<ImportResult> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Không đọc được file: {e}"))?;
    let incoming = backup::import(&text, &password)?;
    let mut added = 0;
    let accounts = commit(&app, |list| {
        if replace {
            added = incoming.len();
            *list = incoming;
        } else {
            added = backup::merge(list, incoming);
        }
        Ok(())
    })?;
    if replace {
        let state = app.state::<AppState>();
        let mut active = state.active.lock().unwrap();
        *active = ActiveState::default();
        let _ = gas_core::state::save(&active);
    }
    Ok(ImportResult { added, accounts })
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_data_dir() -> CmdResult<()> {
    let dir = paths::ensure_data_dir().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let opener = "explorer";
    #[cfg(not(windows))]
    let opener = "xdg-open";
    std::process::Command::new(opener)
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
