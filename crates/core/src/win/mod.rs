//! Windows implementation of [`crate::os`].
//!
//! Only standard Win32 APIs and synthetic keyboard/mouse input are used: no
//! hooks, no DLL injection, no reading other processes' memory. That keeps it
//! compatible with anti-cheat (Vanguard, ACE) the same way a human typing is.

use crate::fields::{pick_login_fields, EditInfo};
use crate::geometry::Rect;
use crate::os::{LoginSpec, ProcessInfo, WindowInfo};
use crate::paths::log_line;
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SendMessageW,
    SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW, WM_GETOBJECT,
};

fn hwnd(handle: isize) -> HWND {
    HWND(handle as *mut core::ffi::c_void)
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

pub fn processes() -> Vec<ProcessInfo> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                out.push(ProcessInfo {
                    pid: entry.th32ProcessID,
                    name: wide_to_string(&entry.szExeFile).to_lowercase(),
                });
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

pub fn process_path(pid: u32) -> Option<PathBuf> {
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL(0), pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        res.ok()?;
        Some(PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
    }
}

pub fn terminate(pid: u32) -> bool {
    use windows::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE,
    };
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, BOOL(0), pid) else {
            return false;
        };
        let ok = TerminateProcess(handle, 1).is_ok();
        if ok {
            let _ = WaitForSingleObject(handle, 3000);
        }
        let _ = CloseHandle(handle);
        ok
    }
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

pub fn top_windows() -> Vec<WindowInfo> {
    struct Ctx {
        out: Vec<WindowInfo>,
        me: u32,
    }

    unsafe extern "system" fn callback(h: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut Ctx);
        if !IsWindowVisible(h).as_bool() {
            return BOOL(1);
        }
        let minimized = IsIconic(h).as_bool();
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid == 0 || pid == ctx.me {
            return BOOL(1);
        }
        let handle = h.0 as isize;
        let client = client_rect(handle).unwrap_or_default();
        if !minimized && (client.width() <= 0 || client.height() <= 0) {
            return BOOL(1);
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(h, &mut buf);
        let title = if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        };
        ctx.out.push(WindowInfo { handle, pid, title, client, minimized });
        BOOL(1)
    }

    let mut ctx = Ctx { out: Vec::new(), me: std::process::id() };
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut ctx as *mut Ctx as isize));
    }
    ctx.out
}

pub fn client_rect(handle: isize) -> Option<Rect> {
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    let h = hwnd(handle);
    unsafe {
        let mut rc = RECT::default();
        GetClientRect(h, &mut rc).ok()?;
        let mut origin = POINT { x: 0, y: 0 };
        if !ClientToScreen(h, &mut origin).as_bool() {
            return None;
        }
        Some(Rect {
            left: origin.x,
            top: origin.y,
            right: origin.x + (rc.right - rc.left),
            bottom: origin.y + (rc.bottom - rc.top),
        })
    }
}

pub fn dpi_scale(handle: isize) -> f64 {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    let dpi = unsafe { GetDpiForWindow(hwnd(handle)) };
    if dpi == 0 {
        1.0
    } else {
        dpi as f64 / 96.0
    }
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x, p.y))
}

pub fn focus(handle: isize) -> bool {
    unsafe { force_foreground(hwnd(handle)) }
}

unsafe fn force_foreground(h: HWND) -> bool {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};

    if IsIconic(h).as_bool() {
        let _ = ShowWindow(h, SW_RESTORE);
        sleep(Duration::from_millis(200));
    } else {
        let _ = ShowWindow(h, SW_SHOW);
    }
    let fg = GetForegroundWindow();
    if fg == h {
        return true;
    }
    let target_tid = GetWindowThreadProcessId(h, None);
    let fg_tid = GetWindowThreadProcessId(fg, None);
    let our_tid = GetCurrentThreadId();

    let _ = AttachThreadInput(our_tid, target_tid, true);
    if fg_tid != 0 && fg_tid != target_tid {
        let _ = AttachThreadInput(fg_tid, target_tid, true);
    }
    let _ = BringWindowToTop(h);
    let ok = SetForegroundWindow(h).as_bool();
    if fg_tid != 0 && fg_tid != target_tid {
        let _ = AttachThreadInput(fg_tid, target_tid, false);
    }
    let _ = AttachThreadInput(our_tid, target_tid, false);
    ok || GetForegroundWindow() == h
}

/// Windows only lets the foreground process hand focus away; a tapped Alt key
/// lifts that restriction for us.
fn bring_to_front(h: HWND, enigo: &mut Enigo) -> Result<(), String> {
    let _ = enigo.key(Key::Alt, Direction::Press);
    let ok = unsafe { force_foreground(h) };
    let _ = enigo.key(Key::Alt, Direction::Release);
    if !ok && unsafe { GetForegroundWindow() } != h {
        return Err("Không đưa được cửa sổ launcher lên trước".into());
    }
    sleep(Duration::from_millis(120));
    Ok(())
}

fn new_enigo() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| format!("Không khởi tạo được bàn phím ảo: {e}"))
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

fn click_at(enigo: &mut Enigo, (x, y): (i32, i32)) -> Result<(), String> {
    enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(50));
    enigo.button(Button::Left, Direction::Click).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(120));
    Ok(())
}

fn press(enigo: &mut Enigo, key: Key) -> Result<(), String> {
    enigo.key(key, Direction::Click).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(60));
    Ok(())
}

fn ctrl_combo(enigo: &mut Enigo, ch: char) -> Result<(), String> {
    enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
    let r = enigo.key(Key::Unicode(ch), Direction::Click);
    let _ = enigo.key(Key::Control, Direction::Release);
    r.map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(50));
    Ok(())
}

/// Select-all + delete, then type `text` into the focused control.
fn replace_text(enigo: &mut Enigo, text: &str) -> Result<(), String> {
    ctrl_combo(enigo, 'a')?;
    press(enigo, Key::Delete)?;
    enigo.text(text).map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(120));
    Ok(())
}

/// Username → Tab → password → (Enter). Assumes the username field has focus.
fn type_credentials(enigo: &mut Enigo, spec: &LoginSpec) -> Result<(), String> {
    replace_text(enigo, spec.username)?;
    press(enigo, Key::Tab)?;
    sleep(Duration::from_millis(120));
    replace_text(enigo, spec.password)?;
    if spec.submit {
        sleep(Duration::from_millis(150));
        press(enigo, Key::Return)?;
    }
    Ok(())
}

pub fn click_point(handle: isize, pt: (i32, i32)) -> Result<(), String> {
    let h = hwnd(handle);
    let mut enigo = new_enigo()?;
    bring_to_front(h, &mut enigo)?;
    click_at(&mut enigo, pt)
}

pub fn fill_at(handle: isize, pt: (i32, i32), spec: &LoginSpec) -> Result<(), String> {
    let h = hwnd(handle);
    let mut enigo = new_enigo()?;
    bring_to_front(h, &mut enigo)?;
    click_at(&mut enigo, pt)?;
    type_credentials(&mut enigo, spec)
}

/// Tries each candidate point: click, type a marker, copy it back. The first
/// point where the marker round-trips through the clipboard is a text field.
pub fn fill_probe(handle: isize, points: &[(i32, i32)], spec: &LoginSpec) -> Result<bool, String> {
    const MARKER: &str = "gas§probe";
    let h = hwnd(handle);
    let mut enigo = new_enigo()?;
    let saved = clipboard_get();

    let mut hit = None;
    for (i, &pt) in points.iter().enumerate() {
        bring_to_front(h, &mut enigo)?;
        click_at(&mut enigo, pt)?;
        let _ = clipboard_set("");
        replace_text(&mut enigo, MARKER)?;
        ctrl_combo(&mut enigo, 'a')?;
        ctrl_combo(&mut enigo, 'c')?;
        sleep(Duration::from_millis(60));
        if clipboard_get().unwrap_or_default().contains(MARKER) {
            log_line(&format!("probe hit #{i} at {pt:?}"));
            hit = Some(pt);
            // Leave the field clean.
            ctrl_combo(&mut enigo, 'a')?;
            press(&mut enigo, Key::Delete)?;
            break;
        }
    }

    if let Some(text) = saved.as_deref() {
        let _ = clipboard_set(text);
    }
    let Some(pt) = hit else {
        return Ok(false);
    };
    bring_to_front(h, &mut enigo)?;
    click_at(&mut enigo, pt)?;
    type_credentials(&mut enigo, spec)?;
    Ok(true)
}

/// Fast path for launchers that expose real edit controls to UI Automation.
pub fn fill_uia(handle: isize, spec: &LoginSpec) -> Result<bool, String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

    let h = hwnd(handle);
    unsafe {
        // Ask Chromium-based UIs to build their accessibility tree.
        let _ = SendMessageW(h, WM_GETOBJECT, WPARAM(0), LPARAM(-4));
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("UI Automation: {e}"))?;
        let root = automation.ElementFromHandle(h).map_err(|e| format!("UIA root: {e}"))?;
        let edits = uia_edits(&automation, &root)?;
        let infos: Vec<EditInfo> = edits.iter().map(|(_, info)| info.clone()).collect();
        let Some((ui, pi)) = pick_login_fields(&infos) else {
            return Ok(false);
        };
        let (Some(user_pt), Some(pass_pt)) = (center_of(&edits[ui].0), center_of(&edits[pi].0)) else {
            return Ok(false);
        };

        let mut enigo = new_enigo()?;
        bring_to_front(h, &mut enigo)?;
        click_at(&mut enigo, user_pt)?;
        replace_text(&mut enigo, spec.username)?;
        click_at(&mut enigo, pass_pt)?;
        replace_text(&mut enigo, spec.password)?;
        if spec.submit {
            sleep(Duration::from_millis(150));
            press(&mut enigo, Key::Return)?;
        }
        Ok(true)
    }
}

type UiaElement = windows::Win32::UI::Accessibility::IUIAutomationElement;

unsafe fn uia_edits(
    automation: &windows::Win32::UI::Accessibility::IUIAutomation,
    root: &UiaElement,
) -> Result<Vec<(UiaElement, EditInfo)>, String> {
    use windows::core::VARIANT;
    use windows::Win32::UI::Accessibility::{
        TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_EditControlTypeId,
        UIA_IsEnabledPropertyId, UIA_IsOffscreenPropertyId, UIA_IsPasswordPropertyId,
    };

    let truthy = |v: Result<VARIANT, windows::core::Error>, default: bool| -> bool {
        v.ok().map(|v| bool::try_from(&v).unwrap_or(default)).unwrap_or(default)
    };

    let condition = automation
        .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_EditControlTypeId.0))
        .map_err(|e| format!("UIA condition: {e}"))?;
    let all = root
        .FindAll(TreeScope_Descendants, &condition)
        .map_err(|e| format!("UIA FindAll: {e}"))?;
    let len = all.Length().unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..len {
        let Ok(el) = all.GetElement(i) else { continue };
        let name = el.CurrentName().map(|s| s.to_string()).unwrap_or_default();
        let is_password = truthy(el.GetCurrentPropertyValue(UIA_IsPasswordPropertyId), false);
        let offscreen = truthy(el.GetCurrentPropertyValue(UIA_IsOffscreenPropertyId), false);
        let enabled = truthy(el.GetCurrentPropertyValue(UIA_IsEnabledPropertyId), true);
        out.push((el, EditInfo { name, is_password, visible: enabled && !offscreen }));
    }
    Ok(out)
}

unsafe fn center_of(el: &UiaElement) -> Option<(i32, i32)> {
    let r = el.CurrentBoundingRectangle().ok()?;
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w < 20 || h < 10 {
        return None;
    }
    Some((r.left + w / 2, r.top + h / 2))
}

// ---------------------------------------------------------------------------
// Clipboard (text only)
// ---------------------------------------------------------------------------

const CF_UNICODETEXT: u32 = 13;

fn clipboard_get() -> Option<String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    unsafe {
        OpenClipboard(HWND::default()).ok()?;
        let Ok(handle) = GetClipboardData(CF_UNICODETEXT) else {
            let _ = CloseClipboard();
            return None;
        };
        let ptr = GlobalLock(HGLOBAL(handle.0));
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let p = ptr as *const u16;
        let mut len = 0usize;
        while *p.add(len) != 0 && len < 1 << 20 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
        let _ = GlobalUnlock(HGLOBAL(handle.0));
        let _ = CloseClipboard();
        Some(s)
    }
}

fn clipboard_set(text: &str) -> Result<(), String> {
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    unsafe {
        OpenClipboard(HWND::default()).map_err(|e| e.to_string())?;
        let _ = EmptyClipboard();
        let mem = match GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2) {
            Ok(m) => m,
            Err(e) => {
                let _ = CloseClipboard();
                return Err(e.to_string());
            }
        };
        let ptr = GlobalLock(mem);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock lỗi".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());
        let _ = GlobalUnlock(mem);
        let res = SetClipboardData(CF_UNICODETEXT, HANDLE(mem.0));
        let _ = CloseClipboard();
        res.map(|_| ()).map_err(|e| e.to_string())
    }
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
