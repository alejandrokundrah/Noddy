// SPDX-License-Identifier: GPL-3.0-or-later
//! Controlling another computer from its mirror window
//! (`docs/protocol/desktop-control.md`): this PC's pointer, buttons, wheel and
//! physical keys, sent to the computer whose screen the window shows, in order.

use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex, OnceLock},
};

use noddy_core::{ButtonAction, DeviceId, DeviceKind, Error, MouseButton, RemoteInput};

use crate::{
    core_host,
    mirror::{self, Phase, Window},
};

/// The most a wheel notch count may be (the protocol allows more).
const MAX_NOTCHES: f32 = 50.0;

/// Keys (DOM codes) each window has sent down and not yet up, so a window that
/// loses focus or closes can let go of them.
static HELD: LazyLock<Mutex<HashMap<Window, HashSet<&'static str>>>> = LazyLock::new(Default::default);

/// Windows that already asked the other computer to allow control.
static ASKED: LazyLock<Mutex<HashSet<Window>>> = LazyLock::new(Default::default);

/// What goes to the other computer.
enum Message {
    /// The pointer, in a datagram.
    Pointer(f32, f32),
    /// Anything else, on the control stream.
    Input(RemoteInput),
}

/// Whether `device` is a computer, as opposed to a phone.
pub fn is_computer(device: &DeviceId) -> bool {
    core_host::host().hub.read(|s| {
        s.device(device).is_some_and(|d| matches!(d.info.kind, DeviceKind::Desktop | DeviceKind::Laptop))
    })
}

fn showing(window: &Window) -> bool {
    matches!(mirror::phase(window), Some(Phase::Showing))
}

/// A position on the screen, kept on it.
fn fraction(v: f64) -> f32 {
    if v.is_finite() { v.clamp(0.0, 1.0) as f32 } else { 0.0 }
}

fn send(window: Window, message: Message) {
    static QUEUE: OnceLock<tokio::sync::mpsc::UnboundedSender<(Window, Message)>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (queue, mut messages) = tokio::sync::mpsc::unbounded_channel::<(Window, Message)>();
        core_host::spawn(async move {
            while let Some((window, message)) = messages.recv().await {
                let Some(node) = core_host::node() else { continue };
                let result = match message {
                    Message::Pointer(x, y) => node.remote_abs(window.device, x, y).await,
                    Message::Input(input) => node.remote_input(window.device, input).await,
                };
                match result {
                    Ok(()) => {}
                    Err(Error::Denied) => ask_to_allow(window),
                    Err(Error::Unsupported) => mirror::set_notice(
                        window,
                        crate::i18n::tr!("Mirror", "Update Noddy on that computer to control it."),
                    ),
                    Err(e) => tracing::debug!(error = %e, "input didn't reach the computer"),
                }
            }
        });
        queue
    });
    let _ = queue.send((window, message));
}

/// The other computer doesn't allow control yet: asks it to (it shows a prompt)
/// and says so in the window.
fn ask_to_allow(window: Window) {
    let first = ASKED.lock().unwrap_or_else(|e| e.into_inner()).insert(window);
    if !first {
        return;
    }
    mirror::set_notice(
        window,
        crate::i18n::tr!("Mirror", "On the other computer, allow control from this one in Noddy."),
    );
    if let Some(node) = core_host::node() {
        core_host::spawn(async move {
            let _ = node.remote_check(window.device).await;
        });
    }
}

/// Asks the other computer to allow control (the window's "Notify" button).
pub fn request_control(window: Window) {
    ASKED.lock().unwrap_or_else(|e| e.into_inner()).remove(&window);
    ask_to_allow(window);
}

/// The pointer at fractions of the other computer's screen.
pub fn pointer(window: Window, x: f64, y: f64) {
    if showing(&window) {
        send(window, Message::Pointer(fraction(x), fraction(y)));
    }
}

/// A mouse button going down or up where the pointer is. The position goes first
/// on the same stream, so the click lands where it was made.
pub fn button(window: Window, button: &str, down: bool, x: f64, y: f64) {
    if !showing(&window) {
        return;
    }
    let button = match button {
        "left" => MouseButton::Left,
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => return,
    };
    let action = if down { ButtonAction::Down } else { ButtonAction::Up };
    send(window, Message::Input(RemoteInput::Abs { x: fraction(x), y: fraction(y) }));
    send(window, Message::Input(RemoteInput::Button { button, action }));
}

/// The wheel, in notches (positive: down / right).
pub fn scroll(window: Window, dx: f64, dy: f64) {
    if !showing(&window) {
        return;
    }
    let notches = |v: f64| if v.is_finite() { (v as f32).clamp(-MAX_NOTCHES, MAX_NOTCHES) } else { 0.0 };
    send(window, Message::Input(RemoteInput::Scroll { dx: notches(dx), dy: notches(dy) }));
}

/// A physical key going down or up. `qt_key` is the Qt key code, and `keypad`
/// whether it came from the number pad.
pub fn key(window: Window, qt_key: i32, keypad: bool, down: bool) {
    if !showing(&window) {
        return;
    }
    let Some(code) = dom_code(qt_key, keypad) else { return };
    {
        let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
        let keys = held.entry(window).or_default();
        if down {
            keys.insert(code);
        } else {
            keys.remove(code);
        }
    }
    send(window, Message::Input(RemoteInput::KeyEv { code: code.to_owned(), down }));
}

/// Lets go of every key this window still holds down on the other computer
/// (the window lost focus or closed).
pub fn release_all(window: Window) {
    let held = HELD.lock().unwrap_or_else(|e| e.into_inner()).remove(&window).unwrap_or_default();
    for code in held {
        send(window, Message::Input(RemoteInput::KeyEv { code: code.to_owned(), down: false }));
    }
}

const LETTERS: [&str; 26] = [
    "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ", "KeyK", "KeyL", "KeyM",
    "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT", "KeyU", "KeyV", "KeyW", "KeyX", "KeyY", "KeyZ",
];
const DIGITS: [&str; 10] =
    ["Digit0", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9"];
const PAD_DIGITS: [&str; 10] = [
    "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8",
    "Numpad9",
];
const FUNCTION_KEYS: [&str; 24] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15", "F16",
    "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
];

/// The DOM `KeyboardEvent.code` of the physical key behind a Qt key code.
/// A shifted symbol maps to the key it is on (US layout), so a key that goes up
/// after Shift did is the same one that went down.
pub fn dom_code(qt_key: i32, keypad: bool) -> Option<&'static str> {
    Some(match qt_key {
        0x41..=0x5a => LETTERS[(qt_key - 0x41) as usize],
        0x30..=0x39 if keypad => PAD_DIGITS[(qt_key - 0x30) as usize],
        0x30..=0x39 => DIGITS[(qt_key - 0x30) as usize],
        0x21 => "Digit1",
        0x40 => "Digit2",
        0x23 => "Digit3",
        0x24 => "Digit4",
        0x25 => "Digit5",
        0x5e => "Digit6",
        0x26 => "Digit7",
        0x2a if keypad => "NumpadMultiply",
        0x2a => "Digit8",
        0x28 => "Digit9",
        0x29 => "Digit0",
        0x20 => "Space",
        0x2b if keypad => "NumpadAdd",
        0x2b | 0x3d => "Equal",
        0x2d if keypad => "NumpadSubtract",
        0x2d | 0x5f => "Minus",
        0x2e if keypad => "NumpadDecimal",
        0x2e | 0x3e => "Period",
        0x2f if keypad => "NumpadDivide",
        0x2f | 0x3f => "Slash",
        0x2c | 0x3c => "Comma",
        0x3b | 0x3a => "Semicolon",
        0x27 | 0x22 => "Quote",
        0x60 | 0x7e => "Backquote",
        0x5b | 0x7b => "BracketLeft",
        0x5d | 0x7d => "BracketRight",
        0x5c | 0x7c => "Backslash",
        0x0100_0000 => "Escape",
        0x0100_0001 | 0x0100_0002 => "Tab",
        0x0100_0003 => "Backspace",
        0x0100_0004 => "Enter",
        0x0100_0005 => "NumpadEnter",
        0x0100_0006 => "Insert",
        0x0100_0007 => "Delete",
        0x0100_0008 => "Pause",
        0x0100_0009 => "PrintScreen",
        0x0100_0010 => "Home",
        0x0100_0011 => "End",
        0x0100_0012 => "ArrowLeft",
        0x0100_0013 => "ArrowUp",
        0x0100_0014 => "ArrowRight",
        0x0100_0015 => "ArrowDown",
        0x0100_0016 => "PageUp",
        0x0100_0017 => "PageDown",
        0x0100_0020 => "ShiftLeft",
        0x0100_0021 => "ControlLeft",
        0x0100_0022 => "MetaLeft",
        0x0100_0023 => "AltLeft",
        0x0100_0024 => "CapsLock",
        0x0100_0025 => "NumLock",
        0x0100_0026 => "ScrollLock",
        0x0100_0030..=0x0100_0047 => FUNCTION_KEYS[(qt_key - 0x0100_0030) as usize],
        0x0100_0055 => "ContextMenu",
        0x0100_1103 => "AltRight",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_function_keys_map_to_their_keys() {
        assert_eq!(dom_code(0x41, false), Some("KeyA"));
        assert_eq!(dom_code(0x5a, false), Some("KeyZ"));
        assert_eq!(dom_code(0x35, false), Some("Digit5"));
        assert_eq!(dom_code(0x35, true), Some("Numpad5"));
        assert_eq!(dom_code(0x0100_0030, false), Some("F1"));
        assert_eq!(dom_code(0x0100_0047, false), Some("F24"));
        assert_eq!(dom_code(0x0100_0048, false), None);
    }

    #[test]
    fn a_shifted_symbol_is_the_key_it_is_on() {
        assert_eq!(dom_code(0x21, false), Some("Digit1")); // !
        assert_eq!(dom_code(0x40, false), Some("Digit2")); // @
        assert_eq!(dom_code(0x5f, false), Some("Minus")); // _
        assert_eq!(dom_code(0x2b, false), Some("Equal")); // +
        assert_eq!(dom_code(0x3f, false), Some("Slash")); // ?
        // The same characters on the number pad are other keys.
        assert_eq!(dom_code(0x2b, true), Some("NumpadAdd"));
        assert_eq!(dom_code(0x2a, true), Some("NumpadMultiply"));
        assert_eq!(dom_code(0x0100_0005, true), Some("NumpadEnter"));
    }

    #[test]
    fn editing_and_modifier_keys_map() {
        for (qt, code) in [
            (0x0100_0012, "ArrowLeft"),
            (0x0100_0003, "Backspace"),
            (0x0100_0004, "Enter"),
            (0x0100_0002, "Tab"),
            (0x0100_0021, "ControlLeft"),
            (0x0100_1103, "AltRight"),
            (0x20, "Space"),
        ] {
            assert_eq!(dom_code(qt, false), Some(code), "{code}");
        }
        assert_eq!(dom_code(0, false), None);
        assert_eq!(dom_code(0x0100_0099, false), None);
    }

    #[test]
    fn every_code_passes_the_protocols_check() {
        let keypad = [false, true];
        for qt in (0..0x80).chain(0x0100_0000..0x0100_1200) {
            for pad in keypad {
                if let Some(code) = dom_code(qt, pad) {
                    assert!(RemoteInput::KeyEv { code: code.to_owned(), down: true }.is_valid(), "{code}");
                }
            }
        }
    }
}
