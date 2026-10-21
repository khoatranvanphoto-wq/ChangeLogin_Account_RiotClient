//! Native Windows APIs for Riot Client detection, focusing, and login fill.
//! UI Automation locates the actual username/password fields — no layout ratios.

use crate::login_fields::{pick_login_fields, LoginEdit};
use crate::model::Account;
use std::thread::sleep;
use std::time::Duration;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SendMessageW, SetActiveWindow,
    SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW, WM_GETOBJECT,
};

const TARGET_PROCESSES: &[&str] = &[
    "riotclientservices.exe",
    "riotclientux.exe",
    "riotclient.exe",
];

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

    sleep(Duration::from_millis(200));
    if !focus_riot_client_window() {
        return Err("Could not focus the Riot Client window".into());
    }
    sleep(Duration::from_millis(350));

    let hwnd = find_riot_hwnd().ok_or("Riot Client window disappeared")?;
    unsafe {
        // Force Chromium to build its accessibility tree.
        let _ = SendMessageW(hwnd, WM_GETOBJECT, WPARAM(0), LPARAM(-4));
    }

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let _ = enigo.key(Key::Escape, Direction::Click);
    sleep(Duration::from_millis(120));

    if !unsafe { force_foreground(hwnd) } {
        return Err("Could not keep Riot Client in the foreground".into());
    }

    for attempt in 0..8 {
        if attempt > 0 {
            sleep(Duration::from_millis(350));
            let _ = unsafe { force_foreground(hwnd) };
        }
        match fill_via_uia(hwnd, &account.username, &account.password, auto_submit, &mut enigo) {
            Ok(true) => {
                log::info!("auto_login: filled via UI Automation");
                return Ok(());
            }
            Ok(false) => continue,
            Err(e) => log::warn!("auto_login UIA attempt {attempt}: {e}"),
        }
    }

    log::info!("auto_login: UI Automation missed the form, using keyboard fallback");
    fill_via_keyboard(&account.username, &account.password, auto_submit, &mut enigo)?;
    Ok(())
}

fn fill_via_keyboard(
    username: &str,
    password: &str,
    auto_submit: bool,
    enigo: &mut enigo::Enigo,
) -> Result<(), String> {
    use enigo::{Direction, Key, Keyboard};

    // No mouse: Riot focuses the username field on the login screen.
    sleep(Duration::from_millis(80));
    type_into_focused(enigo, username)?;
    enigo
        .key(Key::Tab, Direction::Click)
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(180));
    type_into_focused(enigo, password)?;
    if auto_submit {
        sleep(Duration::from_millis(150));
        enigo
            .key(Key::Return, Direction::Click)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
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
                VARIANT::from(UIA_EditControlTypeId.0),
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
    let _ = SetActiveWindow(hwnd);

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
