//! Pure geometry helpers: window rectangles, calibrated relative points, and
//! the candidate points used to find a login field by probing.

use serde::{Deserialize, Serialize};

/// A rectangle in screen coordinates (physical pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    pub fn area(&self) -> i64 {
        self.width().max(0) as i64 * self.height().max(0) as i64
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// A point stored as a fraction of the window's client area, so it survives
/// moving and resizing the window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RelPoint {
    pub x: f64,
    pub y: f64,
}

pub fn rel_to_abs(rect: Rect, p: RelPoint) -> (i32, i32) {
    let x = rect.left as f64 + p.x.clamp(0.0, 1.0) * rect.width() as f64;
    let y = rect.top as f64 + p.y.clamp(0.0, 1.0) * rect.height() as f64;
    (x.round() as i32, y.round() as i32)
}

pub fn abs_to_rel(rect: Rect, x: i32, y: i32) -> Option<RelPoint> {
    if rect.width() <= 0 || rect.height() <= 0 || !rect.contains(x, y) {
        return None;
    }
    Some(RelPoint {
        x: (x - rect.left) as f64 / rect.width() as f64,
        y: (y - rect.top) as f64 / rect.height() as f64,
    })
}

/// Where to look for the username field when nothing is calibrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeLayout {
    /// Riot Client: the sign-in form sits in a fixed-width column on the left.
    LeftColumn,
    /// Unknown layout (Garena): sweep the middle of the window, top to bottom.
    Centered,
}

/// Candidate click points for the username field, most likely first.
pub fn probe_points(rect: Rect, dpi_scale: f64, layout: ProbeLayout) -> Vec<(i32, i32)> {
    let w = rect.width();
    let h = rect.height();
    if w < 200 || h < 200 {
        return Vec::new();
    }
    let scale = if dpi_scale.is_finite() && dpi_scale > 0.0 { dpi_scale } else { 1.0 };
    let mut pts = Vec::new();
    match layout {
        ProbeLayout::LeftColumn => {
            let col = ((480.0 * scale) as i32)
                .min(w * 45 / 100)
                .max((280.0 * scale) as i32);
            let xs = [col / 2, col / 3, (col * 2) / 3];
            for &yr in &[0.22, 0.26, 0.30, 0.34, 0.38, 0.42, 0.18] {
                for &cx in &xs {
                    pts.push((rect.left + cx, rect.top + (h as f64 * yr) as i32));
                }
            }
        }
        ProbeLayout::Centered => {
            let xs = [0.5, 0.42, 0.58];
            for &yr in &[0.30, 0.35, 0.40, 0.45, 0.50, 0.25, 0.55] {
                for &xr in &xs {
                    pts.push((
                        rect.left + (w as f64 * xr) as i32,
                        rect.top + (h as f64 * yr) as i32,
                    ));
                }
            }
        }
    }
    pts.dedup();
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = Rect { left: 100, top: 50, right: 1100, bottom: 650 };

    #[test]
    fn rel_abs_roundtrip() {
        let p = abs_to_rel(R, 600, 350).unwrap();
        assert!((p.x - 0.5).abs() < 1e-9 && (p.y - 0.5).abs() < 1e-9);
        assert_eq!(rel_to_abs(R, p), (600, 350));
    }

    #[test]
    fn outside_point_is_rejected() {
        assert!(abs_to_rel(R, 50, 60).is_none());
        assert!(abs_to_rel(Rect::default(), 0, 0).is_none());
    }

    #[test]
    fn probe_points_stay_inside_window() {
        for layout in [ProbeLayout::LeftColumn, ProbeLayout::Centered] {
            let pts = probe_points(R, 1.5, layout);
            assert!(!pts.is_empty());
            assert!(pts.iter().all(|&(x, y)| R.contains(x, y)));
        }
        assert!(probe_points(Rect { left: 0, top: 0, right: 100, bottom: 100 }, 1.0, ProbeLayout::Centered).is_empty());
    }
}
