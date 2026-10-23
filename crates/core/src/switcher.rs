//! The switching flow shared by the UI, the tray menu and hotkeys:
//! sign out → start launcher → wait for its window → type credentials →
//! start the game.

use crate::geometry::{abs_to_rel, probe_points, rel_to_abs, ProbeLayout, RelPoint};
use crate::model::{Account, Game, Launcher};
use crate::os::{self, LoginSpec, WindowInfo};
use crate::paths::log_line;
use crate::settings::Settings;
use crate::{garena, riot};
use serde::Serialize;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Closing,
    Launching,
    WaitingWindow,
    SigningIn,
    StartingGame,
    Done,
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub step: Step,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Outcome {
    /// False when the account was already signed in and we only started the game.
    pub signed_in: bool,
    pub game_started: bool,
    /// Non-fatal remarks shown to the user.
    pub notes: Vec<String>,
}

pub type Reporter<'a> = &'a dyn Fn(Progress);

fn say(report: Reporter, step: Step, message: impl Into<String>) {
    let message = message.into();
    log_line(&format!("[{step:?}] {message}"));
    report(Progress { step, message });
}

/// Live status for one launcher.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LauncherStatus {
    pub configured: bool,
    pub client_running: bool,
    pub game_running: bool,
}

pub fn status(launcher: Launcher, s: &Settings) -> LauncherStatus {
    match launcher {
        Launcher::Riot => LauncherStatus {
            configured: riot::resolve_client_path(&s.riot.client_path).is_ok(),
            client_running: riot::client_running(),
            game_running: riot::game_running(),
        },
        Launcher::Garena => LauncherStatus {
            configured: garena::launcher_path(&s.garena).is_ok(),
            client_running: garena::launcher_running(&s.garena),
            game_running: garena::game_running(&s.garena),
        },
    }
}

/// Signs `account` in on its launcher and optionally starts `game`.
/// `active_id` is the account this app last signed in on that launcher.
pub fn switch(
    account: &Account,
    game: Option<Game>,
    s: &Settings,
    active_id: Option<&str>,
    report: Reporter,
) -> Result<Outcome, String> {
    if let Some(g) = game {
        if g.launcher() != account.launcher {
            return Err(format!("{} không thuộc {}", g.display_name(), account.launcher.display_name()));
        }
    }
    log_line(&format!("switch to {} ({:?}) game={game:?}", account.id, account.launcher));
    match account.launcher {
        Launcher::Riot => switch_riot(account, game, s, active_id, report),
        Launcher::Garena => switch_garena(account, game, s, active_id, report),
    }
}

pub fn sign_out(launcher: Launcher, s: &Settings) -> Result<(), String> {
    match launcher {
        Launcher::Riot => riot::sign_out(),
        Launcher::Garena => garena::sign_out(&s.garena),
    }
}

// ---------------------------------------------------------------------------
// Riot
// ---------------------------------------------------------------------------

fn switch_riot(
    account: &Account,
    game: Option<Game>,
    s: &Settings,
    active_id: Option<&str>,
    report: Reporter,
) -> Result<Outcome, String> {
    let client = riot::resolve_client_path(&s.riot.client_path)?;
    let mut outcome = Outcome::default();

    // Already signed in with this account: no need to sign out again.
    if active_id == Some(account.id.as_str()) && riot::client_running() {
        if let Some(g) = game {
            say(report, Step::StartingGame, format!("Tài khoản đang đăng nhập sẵn, mở {}…", g.display_name()));
            riot::launch(&client, Some(g))?;
            outcome.game_started = true;
        } else if let Some(w) = riot::find_window() {
            os::focus(w.handle);
        }
        say(report, Step::Done, "Xong");
        return Ok(outcome);
    }

    if riot::client_running() || riot::game_running() {
        say(report, Step::Closing, "Đang đăng xuất tài khoản hiện tại…");
    } else {
        say(report, Step::Closing, "Đang dọn phiên đăng nhập cũ…");
    }
    riot::sign_out()?;

    say(report, Step::Launching, "Đang mở Riot Client…");
    riot::launch(&client, game)?;

    say(report, Step::WaitingWindow, "Chờ màn hình đăng nhập…");
    let window = os::wait_for(Duration::from_secs(60), riot::find_window)
        .ok_or("Riot Client không hiện cửa sổ sau 60 giây")?;
    sleep(Duration::from_millis(s.riot.ready_delay_ms));

    say(report, Step::SigningIn, "Đang điền tài khoản — đừng chạm chuột/bàn phím");
    let spec = LoginSpec { username: &account.username, password: &account.password, submit: s.auto_submit };
    fill_login(&window, &spec, s.riot.username_point, ProbeLayout::LeftColumn, riot::find_window)?;
    outcome.signed_in = true;

    if let Some(g) = game {
        if s.auto_submit {
            say(report, Step::StartingGame, format!("Đang mở {}…", g.display_name()));
            // The client opens the game by itself once signed in; nudge it once
            // more if nothing shows up (e.g. the login took a while).
            let started = os::wait_until(Duration::from_secs(25), || riot::game_started(g));
            if !started {
                log_line("riot: game not detected, re-sending launch command");
                riot::launch(&client, Some(g))?;
                outcome.game_started =
                    os::wait_until(Duration::from_secs(20), || riot::game_started(g));
                if !outcome.game_started {
                    outcome.notes.push(format!(
                        "Chưa thấy {} khởi động. Nếu đăng nhập thành công, hãy bấm Play trong Riot Client.",
                        g.display_name()
                    ));
                }
            } else {
                outcome.game_started = true;
            }
        } else {
            outcome.notes.push("Đã điền xong. Bấm Đăng nhập trong Riot Client; game sẽ tự mở sau đó.".into());
        }
    }
    say(report, Step::Done, "Xong");
    Ok(outcome)
}

// ---------------------------------------------------------------------------
// Garena
// ---------------------------------------------------------------------------

fn switch_garena(
    account: &Account,
    game: Option<Game>,
    s: &Settings,
    active_id: Option<&str>,
    report: Reporter,
) -> Result<Outcome, String> {
    let gs = &s.garena;
    garena::launcher_path(gs)?;
    let mut outcome = Outcome::default();

    if active_id == Some(account.id.as_str()) && garena::launcher_running(gs) {
        if game.is_some() && !garena::game_running(gs) {
            say(report, Step::StartingGame, "Tài khoản đang đăng nhập sẵn, mở game…");
            outcome.game_started = click_launch_button(s)?;
        } else if let Some(w) = garena::find_window(gs) {
            os::focus(w.handle);
        }
        say(report, Step::Done, "Xong");
        return Ok(outcome);
    }

    say(report, Step::Closing, "Đang đăng xuất tài khoản hiện tại…");
    garena::sign_out(gs)?;

    say(report, Step::Launching, "Đang mở launcher Garena…");
    garena::launch(gs)?;

    say(report, Step::WaitingWindow, "Chờ màn hình đăng nhập…");
    let find = || garena::find_window(gs);
    let window = os::wait_for(Duration::from_secs(90), find)
        .ok_or("Launcher Garena không hiện cửa sổ sau 90 giây")?;
    sleep(Duration::from_millis(gs.ready_delay_ms));

    if let Some(tab) = gs.login_tab_point {
        let w = find().unwrap_or(window.clone());
        os::click_point(w.handle, rel_to_abs(w.client, tab))?;
        sleep(Duration::from_millis(800));
    }

    say(report, Step::SigningIn, "Đang điền tài khoản — đừng chạm chuột/bàn phím");
    let spec = LoginSpec { username: &account.username, password: &account.password, submit: s.auto_submit };
    fill_login(&window, &spec, gs.username_point, ProbeLayout::Centered, find)?;
    outcome.signed_in = true;

    if game == Some(Game::DeltaForce) {
        if !s.auto_submit {
            outcome.notes.push("Đã điền xong. Bấm Đăng nhập rồi Khởi động trong launcher.".into());
        } else if gs.launch_button_point.is_none() {
            outcome.notes.push(
                "Chưa hiệu chỉnh nút \"Khởi động\" nên app không tự mở Delta Force. Vào Cài đặt → Garena để ghi vị trí nút.".into(),
            );
        } else {
            say(report, Step::StartingGame, "Chờ đăng nhập xong rồi mở Delta Force…");
            sleep(Duration::from_millis(gs.launch_delay_ms));
            outcome.game_started = click_launch_button(s)?;
            if !outcome.game_started {
                outcome.notes.push("Đã bấm Khởi động nhưng chưa thấy game chạy. Kiểm tra lại launcher.".into());
            }
        }
    }
    say(report, Step::Done, "Xong");
    Ok(outcome)
}

/// Clicks the calibrated "Start game" button. Returns whether the game is
/// confirmed running (only checkable when game processes are configured).
fn click_launch_button(s: &Settings) -> Result<bool, String> {
    let gs = &s.garena;
    let point = gs
        .launch_button_point
        .ok_or("Chưa hiệu chỉnh vị trí nút Khởi động (Cài đặt → Garena)")?;
    let w = garena::find_window(gs).ok_or("Không thấy cửa sổ launcher Garena")?;
    os::click_point(w.handle, rel_to_abs(w.client, point))?;
    if gs.game_processes.is_empty() {
        return Ok(true);
    }
    Ok(os::wait_until(Duration::from_secs(30), || garena::game_running(gs)))
}

// ---------------------------------------------------------------------------
// Shared login fill
// ---------------------------------------------------------------------------

/// Calibrated point → UI Automation → probing, retrying until the form appears.
fn fill_login(
    first: &WindowInfo,
    spec: &LoginSpec,
    calibrated: Option<RelPoint>,
    layout: ProbeLayout,
    refind: impl Fn() -> Option<WindowInfo>,
) -> Result<(), String> {
    let current = || refind().unwrap_or_else(|| first.clone());

    if let Some(p) = calibrated {
        let w = current();
        log_line("fill: calibrated point");
        return os::fill_at(w.handle, rel_to_abs(w.client, p), spec);
    }

    let deadline = Instant::now() + Duration::from_secs(40);
    let mut attempt = 0;
    loop {
        attempt += 1;
        let w = current();
        match os::fill_uia(w.handle, spec) {
            Ok(true) => {
                log_line("fill: UI Automation");
                return Ok(());
            }
            Ok(false) => {}
            Err(e) => log_line(&format!("fill: UIA error {e}")),
        }
        let points = probe_points(w.client, os::dpi_scale(w.handle), layout);
        if os::fill_probe(w.handle, &points, spec)? {
            log_line(&format!("fill: probe (attempt {attempt})"));
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(
                "Không tìm thấy ô đăng nhập. Hãy để launcher ở màn hình đăng nhập, hoặc hiệu chỉnh vị trí ô tài khoản trong Cài đặt.".into(),
            );
        }
        sleep(Duration::from_secs(2));
    }
}

/// Reads the mouse position as a point relative to the launcher's window.
/// Used by the "record position" buttons in Settings.
pub fn capture_point(launcher: Launcher, s: &Settings) -> Result<RelPoint, String> {
    let window = match launcher {
        Launcher::Riot => riot::find_window(),
        Launcher::Garena => garena::find_window(&s.garena),
    }
    .ok_or_else(|| format!("Không thấy cửa sổ {}. Hãy mở nó trước.", launcher.display_name()))?;
    let (x, y) = os::cursor_pos().ok_or("Không đọc được vị trí chuột")?;
    abs_to_rel(window.client, x, y).ok_or_else(|| {
        format!("Con trỏ chuột phải nằm bên trong cửa sổ {}", launcher.display_name())
    })
}
