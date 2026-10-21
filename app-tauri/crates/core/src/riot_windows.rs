//! Native Windows APIs for Riot Client detection, focusing, and login fill.
//!
//! Current Riot Client is CEF (`Chrome_WidgetWin_1` + a D3D compositor). HTML
//! inputs are **not** exposed as UIA Edit controls, so a keyboard-only fill
//! types into nowhere. We locate the username field by clicking candidate
//! points in the left login column and checking that a text field accepted a
//! probe via the clipboard — independent of a single hardcoded pixel.

use crate::login_fields::{pick_login_fields, LoginEdit};
use crate::model::Account;
use crate::paths::AppPaths;
use std::thread::sleep;
use std::time::Duration;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SendMessageW, SetForegroundWindow,
    ShowWindow, SW_RESTORE, SW_SHOW, WM_GETOBJECT,
};

const TARGET_PROCESSES: &[&str] = &[
    "riotclientservices.exe",
    "riotclientux.exe",
    "riotclient.exe",
    "riot client.exe",
];

const PROBE: &str = "§ram§";

pub fn is_riot_client_running() -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snap = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(_) => return false,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut found = false;
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let name = wchar_to_string(&entry.szExeFile).to_lowercase();
                if TARGET_PROCESSES.iter().any(|t| name == *t) {
                    found = true;
                    break;
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        found
    }
}

pub fn is_riot_client_window_visible() -> bool {
    find_riot_hwnd().is_some()
}

pub fn focus_riot_client_window() -> bool {
    let Some(hwnd) = find_riot_hwnd() else {
        return false;
    };
    unsafe { force_foreground(hwnd) }
}

pub fn auto_login(account: &Account, auto_submit: bool) -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    if !is_riot_client_running() || !is_riot_client_window_visible() {
        return Err("Riot Client is not running or its window is not visible".into());
    }

    login_log("auto_login start");
    sleep(Duration::from_millis(150));
    if !focus_riot_client_window() {
        return Err("Could not focus the Riot Client window".into());
    }
    sleep(Duration::from_millis(250));

    let hwnd = find_riot_hwnd().ok_or("Riot Client window disappeared")?;
    unsafe {
        let _ = SendMessageW(hwnd, WM_GETOBJECT, WPARAM(0), LPARAM(-4));
    }

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    steal_foreground(hwnd, &mut enigo)?;

    // Fast path: UIA Edit controls (older / a11y-enabled clients).
    match fill_via_uia(hwnd, &account.username, &account.password, auto_submit, &mut enigo) {
        Ok(true) => {
            login_log("filled via UIA");
            return Ok(());
        }
        Ok(false) => login_log("UIA found no login edits (expected on CEF/D3D)"),
        Err(e) => login_log(&format!("UIA error: {e}")),
    }

    fill_via_probe(hwnd, &account.username, &account.password, auto_submit, &mut enigo)
}

fn steal_foreground(hwnd: HWND, enigo: &mut enigo::Enigo) -> Result<(), String> {
    use enigo::{Direction, Key, Keyboard};
    let _ = enigo.key(Key::Alt, Direction::Press);
    let ok = unsafe { force_foreground(hwnd) };
    let _ = enigo.key(Key::Alt, Direction::Release);
    if !ok && unsafe { GetForegroundWindow() } != hwnd {
        return Err("Could not keep Riot Client in the foreground".into());
    }
    Ok(())
}

fn fill_via_probe(
    hwnd: HWND,
    username: &str,
    password: &str,
    auto_submit: bool,
    enigo: &mut enigo::Enigo,
) -> Result<(), String> {
    use enigo::{Button, Coordinate, Direction, Key, Keyboard, Mouse};

    let saved = clipboard_get();
    let points = login_probe_points(hwnd).ok_or("Could not read Riot Client window size")?;
    login_log(&format!("probe {} candidate points", points.len()));

    let mut found = None;
    for (i, (px, py)) in points.iter().copied().enumerate() {
        steal_foreground(hwnd, enigo)?;
        let _ = enigo.move_mouse(px, py, Coordinate::Abs);
        sleep(Duration::from_millis(40));
        let _ = enigo.button(Button::Left, Direction::Click);
        sleep(Duration::from_millis(90));
        let _ = clipboard_set("");
        type_into_focused(enigo, PROBE)?;
        copy_selection(enigo)?;
        sleep(Duration::from_millis(50));
        let got = clipboard_get().unwrap_or_default();
        if got.contains(PROBE) {
            login_log(&format!("probe hit #{i} at {px},{py}"));
            found = Some((px, py));
            break;
        }
        if i == 4 {
            let _ = enigo.key(Key::Escape, Direction::Click);
            sleep(Duration::from_millis(80));
        }
    }

    let Some((px, py)) = found else {
        let _ = saved.as_ref().map(|s| clipboard_set(s));
        login_log("probe failed: no text field accepted input");
        return Err(
            "Không tìm thấy ô đăng nhập. Hãy để Riot Client ở màn hình Sign in (chưa đăng nhập)."
                .into(),
        );
    };

    steal_foreground(hwnd, enigo)?;
    let _ = enigo.move_mouse(px, py, Coordinate::Abs);
    sleep(Duration::from_millis(40));
    let _ = enigo.button(Button::Left, Direction::Click);
    sleep(Duration::from_millis(80));
    type_into_focused(enigo, username)?;
    enigo
        .key(Key::Tab, Direction::Click)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(160));
    type_into_focused(enigo, password)?;
    if auto_submit {
        sleep(Duration::from_millis(150));
        enigo
            .key(Key::Return, Direction::Click)
            .map_err(|e| e.to_string())?;
    }
    let _ = saved.as_ref().map(|s| clipboard_set(s));
    login_log("filled via probe-click");
    Ok(())
}

fn login_probe_points(hwnd: HWND) -> Option<Vec<(i32, i32)>> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let mut rc = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rc).ok()? };
    let mut origin = POINT { x: 0, y: 0 };
    if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        return None;
    }
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    if w < 200 || h < 200 {
        return None;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as f64;
    let scale = dpi / 96.0;
    let col = ((480.0 * scale) as i32).min(w * 45 / 100).max((280.0 * scale) as i32);

    let mut pts = Vec::new();
    let xs = [col / 2, col / 3, (col * 2) / 3, (120.0 * scale) as i32];
    let y_ratios = [0.20, 0.24, 0.28, 0.32, 0.36, 0.40];
    for &yr in &y_ratios {
        for &cx in &xs {
            pts.push((origin.x + cx, origin.y + (h as f64 * yr) as i32));
        }
    }
    for &css_y in &[180, 220, 260, 300, 340] {
        let y = (css_y as f64 * scale) as i32;
        if y > 40 && y < h - 40 {
            pts.push((origin.x + col / 2, origin.y + y));
        }
    }
    pts.dedup();
    Some(pts)
}

fn copy_selection(enigo: &mut enigo::Enigo) -> Result<(), String> {
    use enigo::{Direction, Key, Keyboard};
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('a'), Direction::Click)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(30));
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(40));
    Ok(())
}

const CF_UNICODETEXT: u32 = 13;

fn clipboard_get() -> Option<String> {
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    unsafe {
        OpenClipboard(HWND::default()).ok()?;
        let handle = match GetClipboardData(CF_UNICODETEXT) {
            Ok(h) => h,
            Err(_) => {
                let _ = CloseClipboard();
                return None;
            }
        };
        let ptr = GlobalLock(windows::Win32::Foundation::HGLOBAL(handle.0));
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let mut len = 0usize;
        let p = ptr as *const u16;
        while *p.add(len) != 0 {
            len += 1;
            if len > 4096 {
                break;
            }
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
        let _ = GlobalUnlock(windows::Win32::Foundation::HGLOBAL(handle.0));
        let _ = CloseClipboard();
        Some(s)
    }
}

fn clipboard_set(text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    unsafe {
        OpenClipboard(HWND::default()).map_err(|e| e.to_string())?;
        let _ = EmptyClipboard();
        let bytes = wide.len() * 2;
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| {
            let _ = CloseClipboard();
            e.to_string()
        })?;
        let ptr = GlobalLock(h);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());
        let _ = GlobalUnlock(h);
        let r = SetClipboardData(CF_UNICODETEXT, HANDLE(h.0));
        let _ = CloseClipboard();
        r.map(|_| ()).map_err(|e| e.to_string())
    }
}

fn login_log(msg: &str) {
    if let Ok(dir) = AppPaths::ensure_data_dir() {
        let path = dir.join("login.log");
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let line = format!("[{ts}] {msg}\n");
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
    log::info!("auto_login: {msg}");
}

fn type_into_focused(enigo: &mut enigo::Enigo, text: &str) -> Result<(), String> {
    use enigo::Keyboard;
    clear_field(enigo)?;
    enigo.text(text).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(120));
    Ok(())
}

fn clear_field(enigo: &mut enigo::Enigo) -> Result<(), String> {
    use enigo::{Direction, Key, Keyboard};
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('a'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(50));
    enigo
        .key(Key::Delete, Direction::Click)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(80));
    Ok(())
}

fn fill_via_uia(
    hwnd: HWND,
    username: &str,
    password: &str,
    auto_submit: bool,
    enigo: &mut enigo::Enigo,
) -> Result<bool, String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("UI Automation init failed: {e}"))?;
        let root = automation
            .ElementFromHandle(hwnd)
            .map_err(|e| format!("ElementFromHandle: {e}"))?;

        let edits = collect_edit_elements(&automation, &root)?;
        let meta: Vec<LoginEdit> = edits
            .iter()
            .map(|e| LoginEdit {
                name: e.name.clone(),
                is_password: e.is_password,
                offscreen: e.offscreen,
                enabled: e.enabled,
            })
            .collect();
        let Some((ui, pi)) = pick_login_fields(&meta) else {
            return Ok(false);
        };

        focus_and_type(enigo, &edits[ui].element, username)?;
        sleep(Duration::from_millis(160));
        focus_and_type(enigo, &edits[pi].element, password)?;

        if auto_submit {
            sleep(Duration::from_millis(150));
            use enigo::{Direction, Key, Keyboard};
            enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| e.to_string())?;
        }
        Ok(true)
    }
}

struct UiaEdit {
    element: windows::Win32::UI::Accessibility::IUIAutomationElement,
    name: String,
    is_password: bool,
    offscreen: bool,
    enabled: bool,
}

fn collect_edit_elements(
    automation: &windows::Win32::UI::Accessibility::IUIAutomation,
    root: &windows::Win32::UI::Accessibility::IUIAutomationElement,
) -> Result<Vec<UiaEdit>, String> {
    use windows::core::VARIANT;
    use windows::Win32::UI::Accessibility::{
        IUIAutomationElement, TreeScope_Descendants, UIA_ControlTypePropertyId,
        UIA_EditControlTypeId, UIA_IsEnabledPropertyId, UIA_IsOffscreenPropertyId,
        UIA_IsPasswordPropertyId,
    };

    unsafe {
        let condition = automation
            .CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_EditControlTypeId.0),
            )
            .map_err(|e| format!("CreatePropertyCondition: {e}"))?;
        let array = root
            .FindAll(TreeScope_Descendants, &condition)
            .map_err(|e| format!("FindAll: {e}"))?;
        let len = array.Length().unwrap_or(0);
        let mut out = Vec::new();
        for i in 0..len {
            let el: IUIAutomationElement = match array.GetElement(i) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let name = el
                .CurrentName()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let is_password = el
                .GetCurrentPropertyValue(UIA_IsPasswordPropertyId)
                .ok()
                .map(|v| variant_truthy(&v))
                .unwrap_or(false);
            let offscreen = el
                .GetCurrentPropertyValue(UIA_IsOffscreenPropertyId)
                .ok()
                .map(|v| variant_truthy(&v))
                .unwrap_or(false);
            let enabled = el
                .GetCurrentPropertyValue(UIA_IsEnabledPropertyId)
                .ok()
                .map(|v| variant_truthy(&v))
                .unwrap_or(true);
            out.push(UiaEdit {
                element: el,
                name,
                is_password,
                offscreen,
                enabled,
            });
        }
        Ok(out)
    }
}

fn variant_truthy(v: &windows::core::VARIANT) -> bool {
    bool::try_from(v).unwrap_or(false)
}

fn focus_and_type(
    enigo: &mut enigo::Enigo,
    element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
    text: &str,
) -> Result<(), String> {
    use enigo::{Button, Coordinate, Direction, Mouse};

    unsafe {
        if element.SetFocus().is_err() {
            if let Ok(rect) = element.CurrentBoundingRectangle() {
                click_rect(enigo, rect)?;
            }
        } else {
            // CEF often needs a real click on the control itself (its live rect, not a layout %).
            if let Ok(rect) = element.CurrentBoundingRectangle() {
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                if w > 20 && h > 10 {
                    let _ = enigo.move_mouse(
                        rect.left + w / 2,
                        rect.top + h / 2,
                        Coordinate::Abs,
                    );
                    sleep(Duration::from_millis(40));
                    let _ = enigo.button(Button::Left, Direction::Click);
                }
            }
        }
    }
    sleep(Duration::from_millis(180));
    type_into_focused(enigo, text)
}

fn click_rect(enigo: &mut enigo::Enigo, rect: RECT) -> Result<(), String> {
    use enigo::{Button, Coordinate, Direction, Mouse};
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 8 || h <= 8 {
        return Err("empty field rectangle".into());
    }
    enigo
        .move_mouse(rect.left + w / 2, rect.top + h / 2, Coordinate::Abs)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(40));
    enigo
        .button(Button::Left, Direction::Click)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn find_riot_hwnd() -> Option<HWND> {
    struct Ctx {
        best: HWND,
        best_area: i32,
        our_pid: u32,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut Ctx);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == ctx.our_pid {
            return BOOL(1);
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        if !is_riot_client_title(&title) {
            return BOOL(1);
        }
        let mut rc = RECT::default();
        if GetWindowRect(hwnd, &mut rc).is_err() {
            return BOOL(1);
        }
        let area = (rc.right - rc.left).saturating_mul(rc.bottom - rc.top);
        if area > ctx.best_area {
            ctx.best = hwnd;
            ctx.best_area = area;
        }
        BOOL(1)
    }

    let mut ctx = Ctx {
        best: HWND::default(),
        best_area: 0,
        our_pid: std::process::id(),
    };
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut ctx as *mut _ as isize));
    }
    if ctx.best_area > 0 {
        Some(ctx.best)
    } else {
        None
    }
}

fn is_riot_client_title(title: &str) -> bool {
    let t = title.to_lowercase();
    t.contains("riot client") && !t.contains("account manager")
}

unsafe fn force_foreground(hwnd: HWND) -> bool {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};

    if IsIconic(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        sleep(Duration::from_millis(200));
    } else {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }

    let fg = GetForegroundWindow();
    let target_tid = GetWindowThreadProcessId(hwnd, None);
    let fg_tid = GetWindowThreadProcessId(fg, None);
    let our_tid = GetCurrentThreadId();

    let _ = AttachThreadInput(our_tid, target_tid, true);
    if fg_tid != 0 && fg_tid != target_tid {
        let _ = AttachThreadInput(fg_tid, target_tid, true);
    }

    let _ = BringWindowToTop(hwnd);
    let ok = SetForegroundWindow(hwnd).as_bool();

    if fg_tid != 0 && fg_tid != target_tid {
        let _ = AttachThreadInput(fg_tid, target_tid, false);
    }
    let _ = AttachThreadInput(our_tid, target_tid, false);

    ok || GetForegroundWindow() == hwnd
}

fn wchar_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
