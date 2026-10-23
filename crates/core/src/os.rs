//! OS facade. On Windows every call goes to [`crate::win`]; elsewhere the
//! functions are inert so the rest of the crate compiles and tests run.
//!
//! Window handles are passed around as `isize` so they can cross threads.

use crate::geometry::Rect;
#[allow(unused_imports)]
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    /// Lower-case executable file name, e.g. `riotclientservices.exe`.
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub handle: isize,
    pub pid: u32,
    pub title: String,
    /// Client area in screen coordinates (empty while minimized).
    pub client: Rect,
    pub minimized: bool,
}

/// Credentials to type and whether to press Enter afterwards.
pub struct LoginSpec<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub submit: bool,
}

#[cfg(windows)]
pub use crate::win::{
    click_point, client_rect, cursor_pos, dpi_scale, fill_at, fill_probe, fill_uia, focus,
    process_path, processes, terminate, top_windows,
};

#[cfg(not(windows))]
mod stub {
    use super::*;

    const UNSUPPORTED: &str = "Chức năng này chỉ chạy trên Windows";

    pub fn processes() -> Vec<ProcessInfo> {
        Vec::new()
    }
    pub fn process_path(_pid: u32) -> Option<PathBuf> {
        None
    }
    pub fn terminate(_pid: u32) -> bool {
        false
    }
    pub fn top_windows() -> Vec<WindowInfo> {
        Vec::new()
    }
    pub fn client_rect(_handle: isize) -> Option<Rect> {
        None
    }
    pub fn dpi_scale(_handle: isize) -> f64 {
        1.0
    }
    pub fn cursor_pos() -> Option<(i32, i32)> {
        None
    }
    pub fn focus(_handle: isize) -> bool {
        false
    }
    pub fn click_point(_handle: isize, _pt: (i32, i32)) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn fill_at(_handle: isize, _pt: (i32, i32), _spec: &LoginSpec) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn fill_uia(_handle: isize, _spec: &LoginSpec) -> Result<bool, String> {
        Err(UNSUPPORTED.into())
    }
    pub fn fill_probe(
        _handle: isize,
        _points: &[(i32, i32)],
        _spec: &LoginSpec,
    ) -> Result<bool, String> {
        Err(UNSUPPORTED.into())
    }
}

#[cfg(not(windows))]
pub use stub::*;

// ---------------------------------------------------------------------------
// Helpers built on the primitives above (platform independent).
// ---------------------------------------------------------------------------

/// Is any process with one of these (case-insensitive) exe names running?
pub fn any_running(names: &[String]) -> bool {
    let wanted: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    processes().iter().any(|p| wanted.contains(&p.name))
}

/// PIDs whose executable lives inside `dir` (any depth).
pub fn pids_in_dir(dir: &std::path::Path) -> Vec<u32> {
    let dir = normalize_path(dir);
    if dir.is_empty() {
        return Vec::new();
    }
    let me = std::process::id();
    processes()
        .into_iter()
        .filter(|p| p.pid != me && p.pid != 0)
        .filter(|p| {
            process_path(p.pid)
                .map(|path| normalize_path(&path).starts_with(&dir))
                .unwrap_or(false)
        })
        .map(|p| p.pid)
        .collect()
}

fn normalize_path(p: &std::path::Path) -> String {
    let mut s = p.to_string_lossy().replace('/', "\\").to_lowercase();
    if !s.ends_with('\\') {
        s.push('\\');
    }
    s
}

/// Terminates every process matching `names`, then waits (up to `timeout`)
/// until none is left. Returns how many were terminated.
pub fn kill_by_names(names: &[String], timeout: std::time::Duration) -> usize {
    let wanted: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let pids: Vec<u32> = processes()
        .into_iter()
        .filter(|p| wanted.contains(&p.name))
        .map(|p| p.pid)
        .collect();
    let killed = pids.iter().filter(|&&pid| terminate(pid)).count();
    wait_until(timeout, || !any_running(names));
    killed
}

pub fn kill_pids(pids: &[u32]) -> usize {
    pids.iter().filter(|&&pid| terminate(pid)).count()
}

/// Polls `cond` every 250 ms until it is true or `timeout` passes.
pub fn wait_until(timeout: std::time::Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    loop {
        if cond() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// Polls for a value until `timeout`.
pub fn wait_for<T>(timeout: std::time::Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let start = std::time::Instant::now();
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if start.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

/// The largest visible, non-minimized window that satisfies `pred`.
pub fn largest_window(pred: impl Fn(&WindowInfo) -> bool) -> Option<WindowInfo> {
    top_windows()
        .into_iter()
        .filter(|w| !w.minimized && w.client.width() >= 200 && w.client.height() >= 150)
        .filter(|w| pred(w))
        .max_by_key(|w| w.client.area())
}

/// Like [`largest_window`], but restores a minimized match first.
pub fn find_window(pred: impl Fn(&WindowInfo) -> bool) -> Option<WindowInfo> {
    if let Some(w) = largest_window(&pred) {
        return Some(w);
    }
    let minimized: Vec<WindowInfo> =
        top_windows().into_iter().filter(|w| w.minimized && pred(w)).collect();
    if minimized.is_empty() {
        return None;
    }
    for w in &minimized {
        focus(w.handle);
    }
    std::thread::sleep(std::time::Duration::from_millis(400));
    largest_window(&pred)
}

/// Starts an executable detached from this process.
pub fn spawn(path: &std::path::Path, args: &[String]) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Không tìm thấy file: {}", path.display()));
    }
    let mut cmd = std::process::Command::new(path);
    cmd.args(args);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Không mở được {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_adds_trailing_separator() {
        assert_eq!(normalize_path(std::path::Path::new("C:/Garena/DF")), "c:\\garena\\df\\");
        assert_eq!(normalize_path(std::path::Path::new("C:\\A\\")), "c:\\a\\");
    }

    #[test]
    fn wait_until_times_out() {
        assert!(!wait_until(std::time::Duration::from_millis(10), || false));
        assert!(wait_until(std::time::Duration::from_millis(10), || true));
    }
}
