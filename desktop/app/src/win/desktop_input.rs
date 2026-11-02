// SPDX-License-Identifier: GPL-3.0-or-later
//! Desktop-style remote control on Windows: an absolute pointer on a chosen
//! monitor, and physical keys going down and up by DOM `KeyboardEvent.code`
//! (`docs/protocol/desktop-control.md`).
//!
//! Not wired to the protocol yet: nothing calls this until `input.desktop` is
//! handled in the node and `pc_mirror`.

#![allow(dead_code)]

use std::sync::Mutex;

use windows::Win32::UI::{
    Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK,
        MOUSEINPUT, SendInput, VIRTUAL_KEY,
    },
    WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    },
};

/// The most keys held down at once by remote input. More are refused.
pub const MAX_HELD_KEYS: usize = 16;

/// A rectangle of the virtual desktop in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// Keys (`vk`, `extended`) held down by remote input, so a session that ends
/// can let go of them.
static HELD_KEYS: Mutex<Vec<(u16, bool)>> = Mutex::new(Vec::new());

fn send_inputs(inputs: &[INPUT]) {
    if inputs.is_empty() {
        return;
    }
    // SAFETY: `inputs` is a valid slice of `INPUT` structs.
    unsafe {
        let _ = SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn mouse_input(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx, dy, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn key_input(vk: u16, extended: bool, up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

/// The whole virtual desktop (every monitor), in physical pixels when the
/// process is DPI aware, as the app is.
pub fn virtual_desktop() -> ScreenRect {
    // SAFETY: plain system metric queries.
    unsafe {
        ScreenRect {
            left: GetSystemMetrics(SM_XVIRTUALSCREEN),
            top: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    }
}

/// The primary monitor, in physical pixels. It always sits at the origin of the
/// virtual desktop. This is the monitor `pc_mirror` streams today.
pub fn primary_monitor() -> ScreenRect {
    // SAFETY: plain system metric queries.
    unsafe {
        ScreenRect {
            left: 0,
            top: 0,
            width: GetSystemMetrics(SM_CXSCREEN),
            height: GetSystemMetrics(SM_CYSCREEN),
        }
    }
}

/// Turns a point given as fractions (`0.0..=1.0`) of `monitor` into the
/// `0..=65535` range `SendInput` takes for the whole virtual desktop `virt`.
/// `None` for a point that isn't finite or a rectangle with no area.
pub fn to_virtual_desk(monitor: ScreenRect, virt: ScreenRect, x: f32, y: f32) -> Option<(i32, i32)> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    if monitor.width <= 0 || monitor.height <= 0 || virt.width <= 0 || virt.height <= 0 {
        return None;
    }
    let fx = f64::from(x.clamp(0.0, 1.0));
    let fy = f64::from(y.clamp(0.0, 1.0));
    // The last pixel of the monitor is at `width - 1`, so 1.0 lands on it.
    let px = f64::from(monitor.left) + fx * f64::from(monitor.width - 1);
    let py = f64::from(monitor.top) + fy * f64::from(monitor.height - 1);
    let nx = (px - f64::from(virt.left)) / f64::from((virt.width - 1).max(1)) * 65535.0;
    let ny = (py - f64::from(virt.top)) / f64::from((virt.height - 1).max(1)) * 65535.0;
    Some((nx.round().clamp(0.0, 65535.0) as i32, ny.round().clamp(0.0, 65535.0) as i32))
}

/// Moves the pointer to fractions `(x, y)` of `monitor`, wherever that monitor
/// sits on the virtual desktop.
pub fn move_absolute_on(monitor: ScreenRect, x: f32, y: f32) {
    let Some((ax, ay)) = to_virtual_desk(monitor, virtual_desktop(), x, y) else {
        return;
    };
    send_inputs(&[mouse_input(ax, ay, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK)]);
}

/// Presses (`down`) or releases a physical key named by its DOM
/// `KeyboardEvent.code`. False when the code is unknown or too many keys are
/// already held, in which case nothing is sent.
pub fn key_event(code: &str, down: bool) -> bool {
    let Some((vk, extended)) = dom_code_to_vk(code) else {
        return false;
    };
    {
        let mut held = HELD_KEYS.lock().unwrap_or_else(|e| e.into_inner());
        if down {
            // A key already held is an auto-repeat: it goes down again.
            if !held.iter().any(|&(k, _)| k == vk) {
                if held.len() >= MAX_HELD_KEYS {
                    return false;
                }
                held.push((vk, extended));
            }
        } else {
            held.retain(|&(k, _)| k != vk);
        }
    }
    send_inputs(&[key_input(vk, extended, !down)]);
    true
}

/// Releases every key still held down by remote input (the session ended, the
/// permission was turned off, or the connection dropped).
pub fn release_held_keys() {
    let held = std::mem::take(&mut *HELD_KEYS.lock().unwrap_or_else(|e| e.into_inner()));
    let inputs: Vec<INPUT> = held.iter().rev().map(|&(vk, extended)| key_input(vk, extended, true)).collect();
    send_inputs(&inputs);
}

/// Maps a DOM `KeyboardEvent.code` to a Windows virtual-key code and whether
/// the key is an "extended" one (the arrow and edit clusters, the right-hand
/// Ctrl and Alt, the Windows keys, Numpad `/` and Enter).
pub fn dom_code_to_vk(code: &str) -> Option<(u16, bool)> {
    let bytes = code.as_bytes();
    if bytes.len() == 4 && code.starts_with("Key") && bytes[3].is_ascii_uppercase() {
        return Some((u16::from(bytes[3]), false)); // VK_A..VK_Z are 'A'..'Z'
    }
    if bytes.len() == 6 && code.starts_with("Digit") && bytes[5].is_ascii_digit() {
        return Some((u16::from(bytes[5]), false)); // VK_0..VK_9 are '0'..'9'
    }
    if bytes.len() == 7 && code.starts_with("Numpad") && bytes[6].is_ascii_digit() {
        return Some((0x60 + u16::from(bytes[6] - b'0'), false)); // VK_NUMPAD0..9
    }
    if let Some(n) = code.strip_prefix('F')
        && !n.starts_with('0')
        && let Ok(n) = n.parse::<u16>()
        && (1..=24).contains(&n)
    {
        return Some((0x6F + n, false)); // VK_F1..VK_F24
    }
    Some(match code {
        "Backspace" => (0x08, false),
        "Tab" => (0x09, false),
        "Enter" => (0x0D, false),
        "NumpadEnter" => (0x0D, true),
        "Pause" => (0x13, false),
        "CapsLock" => (0x14, false),
        "Escape" => (0x1B, false),
        "Space" => (0x20, false),
        "PageUp" => (0x21, true),
        "PageDown" => (0x22, true),
        "End" => (0x23, true),
        "Home" => (0x24, true),
        "ArrowLeft" => (0x25, true),
        "ArrowUp" => (0x26, true),
        "ArrowRight" => (0x27, true),
        "ArrowDown" => (0x28, true),
        "PrintScreen" => (0x2C, true),
        "Insert" => (0x2D, true),
        "Delete" => (0x2E, true),
        "MetaLeft" | "OSLeft" => (0x5B, true),
        "MetaRight" | "OSRight" => (0x5C, true),
        "ContextMenu" => (0x5D, true),
        "NumpadMultiply" => (0x6A, false),
        "NumpadAdd" => (0x6B, false),
        "NumpadSubtract" => (0x6D, false),
        "NumpadDecimal" => (0x6E, false),
        "NumpadDivide" => (0x6F, true),
        "NumLock" => (0x90, true),
        "ScrollLock" => (0x91, false),
        "ShiftLeft" => (0xA0, false),
        "ShiftRight" => (0xA1, false),
        "ControlLeft" => (0xA2, false),
        "ControlRight" => (0xA3, true),
        "AltLeft" => (0xA4, false),
        "AltRight" => (0xA5, true),
        "Semicolon" => (0xBA, false),
        "Equal" => (0xBB, false),
        "Comma" => (0xBC, false),
        "Minus" => (0xBD, false),
        "Period" => (0xBE, false),
        "Slash" => (0xBF, false),
        "Backquote" => (0xC0, false),
        "BracketLeft" => (0xDB, false),
        "Backslash" => (0xDC, false),
        "BracketRight" => (0xDD, false),
        "Quote" => (0xDE, false),
        "IntlBackslash" => (0xE2, false),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: ScreenRect = ScreenRect { left: 0, top: 0, width: 1920, height: 1080 };

    #[test]
    fn maps_letters_digits_numpad_and_function_keys() {
        assert_eq!(dom_code_to_vk("KeyA"), Some((0x41, false)));
        assert_eq!(dom_code_to_vk("KeyZ"), Some((0x5A, false)));
        assert_eq!(dom_code_to_vk("Digit7"), Some((0x37, false)));
        assert_eq!(dom_code_to_vk("Numpad5"), Some((0x65, false)));
        assert_eq!(dom_code_to_vk("F1"), Some((0x70, false)));
        assert_eq!(dom_code_to_vk("F12"), Some((0x7B, false)));
        assert_eq!(dom_code_to_vk("F24"), Some((0x87, false)));
    }

    #[test]
    fn edit_clusters_and_right_hand_modifiers_are_extended() {
        assert_eq!(dom_code_to_vk("ArrowLeft"), Some((0x25, true)));
        assert_eq!(dom_code_to_vk("Delete"), Some((0x2E, true)));
        assert_eq!(dom_code_to_vk("ControlRight"), Some((0xA3, true)));
        assert_eq!(dom_code_to_vk("ControlLeft"), Some((0xA2, false)));
        assert_eq!(dom_code_to_vk("NumpadEnter"), Some((0x0D, true)));
        assert_eq!(dom_code_to_vk("Enter"), Some((0x0D, false)));
    }

    #[test]
    fn unknown_codes_are_refused() {
        for code in ["", "Key", "Keya", "KeyAB", "Digit10", "Numpad", "F0", "F01", "F25", "Bogus"] {
            assert_eq!(dom_code_to_vk(code), None, "{code}");
        }
    }

    #[test]
    fn one_monitor_spans_the_whole_range() {
        assert_eq!(to_virtual_desk(ONE, ONE, 0.0, 0.0), Some((0, 0)));
        assert_eq!(to_virtual_desk(ONE, ONE, 1.0, 1.0), Some((65535, 65535)));
        assert_eq!(to_virtual_desk(ONE, ONE, 0.5, 0.5), Some((32768, 32768)));
        // Out-of-range fractions stop at the edge.
        assert_eq!(to_virtual_desk(ONE, ONE, -3.0, 9.0), Some((0, 65535)));
    }

    #[test]
    fn a_monitor_to_the_left_maps_into_the_left_half() {
        let left = ScreenRect { left: -1920, top: 0, width: 1920, height: 1080 };
        let virt = ScreenRect { left: -1920, top: 0, width: 3840, height: 1080 };
        assert_eq!(to_virtual_desk(left, virt, 0.0, 0.0), Some((0, 0)));
        let (right_edge, _) = to_virtual_desk(left, virt, 1.0, 0.0).unwrap();
        assert!(right_edge < 32768, "{right_edge}");
        // The primary monitor, to the right of it, ends at the far edge.
        assert_eq!(to_virtual_desk(ONE, virt, 1.0, 1.0), Some((65535, 65535)));
        let (primary_left_edge, _) = to_virtual_desk(ONE, virt, 0.0, 0.0).unwrap();
        assert!(primary_left_edge > right_edge);
    }

    #[test]
    fn bad_points_and_empty_rectangles_are_refused() {
        assert_eq!(to_virtual_desk(ONE, ONE, f32::NAN, 0.0), None);
        assert_eq!(to_virtual_desk(ONE, ONE, 0.0, f32::INFINITY), None);
        let empty = ScreenRect { left: 0, top: 0, width: 0, height: 1080 };
        assert_eq!(to_virtual_desk(empty, ONE, 0.5, 0.5), None);
        assert_eq!(to_virtual_desk(ONE, empty, 0.5, 0.5), None);
    }
}
