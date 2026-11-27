// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh: unified control over every paired device at once.
//!
//! When Mesh is enabled the cursor flows between the PC and paired devices
//! automatically — crossing a configured screen edge triggers `input_share::enter_remote`
//! for the neighbouring device, and crossing back (or reaching its far edge)
//! calls `leave_remote`. The clipboard and notification pool are already unified
//! by the existing `clipboard::on_event` rebroadcast and the `NotificationList`
//! "All devices" mode; this module only manages the cursor-flow monitor thread.

use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use noddy_core::{DeviceId, NodeEvent};

use crate::{
    input_share::{self, ScreenEdge},
    settings::MeshDeviceLayout,
};

// ── Shared state ────────────────────────────────────────────────────────────

static RUNNING: AtomicBool = AtomicBool::new(false);
static LAYOUT: OnceLock<Mutex<Vec<MeshDeviceLayout>>> = OnceLock::new();

fn layout_slot() -> &'static Mutex<Vec<MeshDeviceLayout>> {
    LAYOUT.get_or_init(|| Mutex::new(Vec::new()))
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Updates the in-memory layout used by the monitor thread.
pub fn update_layout(layout: Vec<MeshDeviceLayout>) {
    *layout_slot().lock().unwrap_or_else(|e| e.into_inner()) = layout;
}

/// Starts the cursor-edge monitor thread if it is not already running.
/// Safe to call multiple times; only one thread runs at a time.
pub fn start_monitoring() {
    if RUNNING.swap(true, Ordering::AcqRel) {
        return; // already running
    }
    std::thread::Builder::new().name("mesh-monitor".into()).spawn(monitor_loop).ok();
}

/// Signals the monitor thread to stop at its next tick.
pub fn stop_monitoring() {
    RUNNING.store(false, Ordering::Release);
    // If input was flowing to a remote, hand it back to the PC.
    input_share::leave_remote();
}

/// Called from the main event loop for every `NodeEvent` so Mesh can react to
/// device removals and link changes.
pub fn on_event(event: &NodeEvent) {
    match event {
        NodeEvent::DeviceRemoved(id) => on_device_gone(id),
        NodeEvent::LinkChanged { device, link } => {
            if matches!(link, noddy_core::LinkState::Offline { .. }) {
                on_device_gone(device);
            }
        }
        _ => {}
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn on_device_gone(id: &DeviceId) {
    // If this device is currently the active remote, hand control back to the PC.
    if input_share::active_remote() == Some(*id) {
        input_share::leave_remote();
    }
    // Remove it from the layout so the monitor won't try to enter it.
    let mut lock = layout_slot().lock().unwrap_or_else(|e| e.into_inner());
    let id_hex = id.to_string();
    lock.retain(|d| d.device_id != id_hex);
}

/// Returns the `DeviceId` of the first layout entry on `edge`, or `None`.
fn device_on_edge(layout: &[MeshDeviceLayout], edge: ScreenEdge) -> Option<DeviceId> {
    let target = match edge {
        ScreenEdge::Left => crate::settings::MeshEdge::Left,
        ScreenEdge::Right => crate::settings::MeshEdge::Right,
        ScreenEdge::Top => crate::settings::MeshEdge::Top,
        ScreenEdge::Bottom => crate::settings::MeshEdge::Bottom,
    };
    // Pick the entry with the lowest `order` on that edge.
    let mut candidates: Vec<&MeshDeviceLayout> = layout.iter().filter(|d| d.edge == target).collect();
    candidates.sort_by_key(|d| d.order);
    candidates.first().and_then(|d| d.device_id.parse().ok())
}

/// Returns the primary monitor bounds `(left, top, right, bottom)` in screen pixels.
#[cfg(windows)]
fn primary_monitor_bounds() -> (i32, i32, i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO};
    use windows::Win32::UI::WindowsAndMessaging::MonitorFromPoint;

    // `MonitorFromPoint` with `MONITOR_DEFAULTTOPRIMARY` always returns a handle.
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() } {
        let r = info.rcMonitor;
        (r.left, r.top, r.right, r.bottom)
    } else {
        (0, 0, 1920, 1080) // safe fallback
    }
}

#[cfg(not(windows))]
fn primary_monitor_bounds() -> (i32, i32, i32, i32) {
    (0, 0, 1920, 1080)
}

/// Returns the current cursor position `(x, y)` in screen pixels.
#[cfg(windows)]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt).ok().map(|_| (pt.x, pt.y)) }
}

#[cfg(not(windows))]
fn cursor_pos() -> Option<(i32, i32)> {
    None
}

/// The background thread: polls cursor position every ~16 ms and calls
/// `enter_remote` / `leave_remote` based on the configured layout.
fn monitor_loop() {
    const TICK: std::time::Duration = std::time::Duration::from_millis(16);

    while RUNNING.load(Ordering::Relaxed) {
        std::thread::sleep(TICK);

        let (cx, cy) = match cursor_pos() {
            Some(pos) => pos,
            None => continue,
        };

        let layout = layout_slot().lock().unwrap_or_else(|e| e.into_inner()).clone();
        if layout.is_empty() {
            continue;
        }

        let (left, top, right, bottom) = primary_monitor_bounds();
        let current_remote = input_share::active_remote();

        if let Some(active) = current_remote {
            // Currently controlling a remote device.
            // The mirror window captures the cursor, so cursor position on the
            // PC monitor isn't meaningful while remote. We rely on the mirror
            // window losing/gaining focus to call leave_remote; here we just
            // watch for a leave that the user triggered by pressing Escape (or
            // by moving out of the mirror window, handled by pc_mirror).
            // If `active` is no longer in the layout, leave immediately.
            let active_hex = active.to_string();
            if !layout.iter().any(|d| d.device_id == active_hex) {
                input_share::leave_remote();
            }
            let _ = active; // suppress unused warning
        } else {
            // Currently on the PC monitor. Check whether the cursor hit an edge
            // that has a device assigned.
            let edges = [
                (
                    ScreenEdge::Left,
                    input_share::is_at_screen_edge(cx, cy, left, top, right, bottom, ScreenEdge::Left),
                ),
                (
                    ScreenEdge::Right,
                    input_share::is_at_screen_edge(cx, cy, left, top, right, bottom, ScreenEdge::Right),
                ),
                (
                    ScreenEdge::Top,
                    input_share::is_at_screen_edge(cx, cy, left, top, right, bottom, ScreenEdge::Top),
                ),
                (
                    ScreenEdge::Bottom,
                    input_share::is_at_screen_edge(cx, cy, left, top, right, bottom, ScreenEdge::Bottom),
                ),
            ];
            for (edge, hit) in edges {
                if hit {
                    if let Some(peer) = device_on_edge(&layout, edge) {
                        input_share::enter_remote(peer);
                        break;
                    }
                }
            }
        }
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{MeshDeviceLayout, MeshEdge};

    fn layout_with(edge: MeshEdge) -> Vec<MeshDeviceLayout> {
        vec![MeshDeviceLayout { device_id: "aa".repeat(16), edge, order: 0 }]
    }

    #[test]
    fn device_on_edge_returns_device_for_matching_edge() {
        let layout = layout_with(MeshEdge::Right);
        assert!(device_on_edge(&layout, ScreenEdge::Right).is_some());
        assert!(device_on_edge(&layout, ScreenEdge::Left).is_none());
        assert!(device_on_edge(&layout, ScreenEdge::Top).is_none());
        assert!(device_on_edge(&layout, ScreenEdge::Bottom).is_none());
    }

    #[test]
    fn device_on_edge_picks_lowest_order_when_multiple_on_same_edge() {
        let id_first = "aa".repeat(16);
        let id_second = "bb".repeat(16);
        let layout = vec![
            MeshDeviceLayout { device_id: id_second.clone(), edge: MeshEdge::Right, order: 1 },
            MeshDeviceLayout { device_id: id_first.clone(), edge: MeshEdge::Right, order: 0 },
        ];
        let got: String =
            device_on_edge(&layout, ScreenEdge::Right).map(|d| d.to_string()).unwrap_or_default();
        // order 0 device wins
        assert_eq!(got, id_first);
    }

    #[test]
    fn mesh_layout_update_round_trips() {
        let layout = layout_with(MeshEdge::Left);
        update_layout(layout.clone());
        let read = layout_slot().lock().unwrap().clone();
        assert_eq!(read, layout);
    }
}
