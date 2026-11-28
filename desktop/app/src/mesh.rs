// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh: one mouse and keyboard for this computer and the computers next to it.
//!
//! With Mesh on, pushing the pointer against a screen edge that has a computer
//! assigned to it (Settings › Mesh) hands this computer's mouse and keyboard to
//! that computer: the pointer appears at its matching edge, and clicks, the
//! wheel and keys go there, using desktop control
//! (`docs/protocol/desktop-control.md`: `abs`, `button`, `scroll`, `keyev`).
//! Moving back across that edge returns them here, as does Ctrl+Alt+M, and so
//! does the other computer going offline or refusing control. Keys and buttons
//! still held over there are let go when control comes back.
//!
//! The other computer must accept desktop input (a Windows PC so far) and allow
//! remote control from this one, as for Control PC. Phones take no mouse or
//! keyboard. Capturing this computer's mouse and keyboard is Windows only so far.
//! The clipboard and notifications are shared by `clipboard::on_event` and the
//! "All devices" notification list; the clipboard also follows the pointer
//! (`input_share::enter_remote`), files you copied included: they are sent to
//! the computer the pointer crosses to. Ctrl+Alt+V sends what you copied (files,
//! text or a picture) to the computer that has the pointer, or the first one in
//! the layout.
//!
//! Dragging a window by its title bar to such an edge offers it to that
//! computer (`window.offer`): it opens there, in a window of its own that shows
//! this one's window and sends its mouse and keyboard back (the same as opening
//! one of its app windows from there). The pointer stays here, and the window
//! stays here too.

// Capturing the mouse and keyboard is Windows only so far; elsewhere the
// pointer logic is built (and tested) but nothing drives it.
#![cfg_attr(not(windows), allow(dead_code))]

use std::{
    collections::HashSet,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use noddy_core::{ButtonAction, DeviceId, Error, MouseButton, NodeEvent, RemoteInput};

use crate::{
    core_host,
    input_share::{self, ScreenEdge},
    settings::{MeshDeviceLayout, MeshEdge},
};

// ── Layout ──────────────────────────────────────────────────────────────────

static LAYOUT: LazyLock<Mutex<Vec<MeshDeviceLayout>>> = LazyLock::new(Default::default);
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Updates the layout the edges are checked against.
pub fn update_layout(layout: Vec<MeshDeviceLayout>) {
    *LAYOUT.lock().unwrap_or_else(|e| e.into_inner()) = layout;
}

/// The devices on `edge`, lowest `order` first.
fn devices_on_edge(layout: &[MeshDeviceLayout], edge: ScreenEdge) -> Vec<DeviceId> {
    let target = match edge {
        ScreenEdge::Left => MeshEdge::Left,
        ScreenEdge::Right => MeshEdge::Right,
        ScreenEdge::Top => MeshEdge::Top,
        ScreenEdge::Bottom => MeshEdge::Bottom,
    };
    let mut on_edge: Vec<&MeshDeviceLayout> = layout.iter().filter(|d| d.edge == target).collect();
    on_edge.sort_by_key(|d| d.order);
    on_edge.iter().filter_map(|d| d.device_id.parse().ok()).collect()
}

/// The computer a copy/paste hotkey sends to when the pointer is here: the
/// first in the layout (lowest `order`, then Left, Right, Top, Bottom).
fn default_target(layout: &[MeshDeviceLayout]) -> Option<DeviceId> {
    let rank = |edge: MeshEdge| match edge {
        MeshEdge::Left => 0,
        MeshEdge::Right => 1,
        MeshEdge::Top => 2,
        MeshEdge::Bottom => 3,
    };
    layout
        .iter()
        .filter_map(|d| Some(((d.order, rank(d.edge)), d.device_id.parse::<DeviceId>().ok()?)))
        .min_by_key(|(key, _)| *key)
        .map(|(_, id)| id)
}

/// Ctrl+Alt+V: sends what is copied on this computer (files, text or a
/// picture) to the computer that has the pointer, or else to the first one in
/// the layout. The files land in its Noddy folder and are put on its clipboard,
/// ready to paste.
pub fn send_copied() {
    let target = away_at().or_else(|| default_target(&LAYOUT.lock().unwrap_or_else(|e| e.into_inner())));
    match target {
        // Off the hook's thread: this reads the clipboard and the device list.
        Some(peer) => {
            std::thread::spawn(move || crate::clipboard::send_now(peer));
        }
        None => crate::bridge::app::show_message(crate::i18n::tr!(
            "Mesh",
            "Put a computer next to this screen in Settings › Mesh to send it what you copied."
        )),
    }
}

/// Offers the window being dragged (`hwnd`, titled `title`) to `peer`: it
/// opens there as a window of its own, showing this one. Runs off the hook's
/// thread.
fn offer_window(peer: DeviceId, hwnd: isize, title: String) {
    core_host::spawn(async move {
        let Some(node) = core_host::node() else { return };
        match node.offer_window(peer, window_key(hwnd), title).await {
            Ok(()) => {}
            Err(Error::Unsupported) => crate::bridge::app::show_message(crate::i18n::tr!(
                "Mesh",
                "That computer can't open windows from this one yet. Update Noddy there."
            )),
            Err(e) => tracing::debug!(error = %e, "Mesh couldn't offer the window"),
        }
    });
}

/// A window's ID in `mirror.apps` (`docs/protocol/mirror.md` §6).
fn window_key(hwnd: isize) -> String {
    format!("win:{hwnd}")
}

// ── Who can take the pointer ────────────────────────────────────────────────

/// Computers that just refused control or can't take it, and until when they
/// aren't tried again (so the pointer isn't caught at that edge in a loop).
static COOLDOWN: LazyLock<Mutex<Vec<(DeviceId, Instant)>>> = LazyLock::new(Default::default);
const COOLDOWN_FOR: Duration = Duration::from_secs(5);

fn cooling_down(peer: &DeviceId) -> bool {
    let mut list = COOLDOWN.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    list.retain(|(_, until)| *until > now);
    list.iter().any(|(id, _)| id == peer)
}

fn cool_down(peer: DeviceId) {
    COOLDOWN.lock().unwrap_or_else(|e| e.into_inner()).push((peer, Instant::now() + COOLDOWN_FOR));
}

/// Whether the pointer can go to `peer` now: a computer that's connected.
fn can_take_pointer(peer: &DeviceId) -> bool {
    let online = core_host::host()
        .hub
        .read(|s| s.device(peer).is_some_and(|d| matches!(d.link, noddy_core::LinkState::Online { .. })));
    online && crate::desktop_control::is_computer(peer) && !cooling_down(peer)
}

/// The computer that takes the pointer at `edge`, if any.
fn target_at(edge: ScreenEdge) -> Option<DeviceId> {
    let layout = LAYOUT.lock().unwrap_or_else(|e| e.into_inner()).clone();
    devices_on_edge(&layout, edge).into_iter().find(can_take_pointer)
}

// ── Where the pointer is ────────────────────────────────────────────────────

/// The pointer while it's on another computer: where it is there, as fractions
/// of that computer's screen, and which edge here it went through.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Away {
    peer: DeviceId,
    /// The edge of this computer's screen the pointer left through.
    edge: ScreenEdge,
    x: f64,
    y: f64,
    /// That computer's screen in pixels, to turn motion into fractions. Taken to
    /// be the size of this one's (the protocol doesn't say yet).
    width: f64,
    height: f64,
}

/// What a pointer movement while away comes to.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    /// The pointer is now at these fractions of the other computer's screen.
    There(f32, f32),
    /// It came back across the edge it left through, at this fraction along
    /// that edge.
    Back(f64),
}

impl Away {
    /// The pointer crossed `edge` of this computer's screen at `along` (the
    /// fraction along that edge: top to bottom for left and right, left to right
    /// for top and bottom), onto a screen `width` × `height`.
    fn enter(peer: DeviceId, edge: ScreenEdge, along: f64, width: f64, height: f64) -> Away {
        let along = along.clamp(0.0, 1.0);
        let (x, y) = match edge {
            ScreenEdge::Right => (0.0, along),
            ScreenEdge::Left => (1.0, along),
            ScreenEdge::Bottom => (along, 0.0),
            ScreenEdge::Top => (along, 1.0),
        };
        Away { peer, edge, x, y, width: width.max(1.0), height: height.max(1.0) }
    }

    /// Moves the pointer by `(dx, dy)` pixels.
    fn moved(&mut self, dx: f64, dy: f64) -> Step {
        self.x += dx / self.width;
        self.y += dy / self.height;
        let back = match self.edge {
            ScreenEdge::Right => self.x < 0.0,
            ScreenEdge::Left => self.x > 1.0,
            ScreenEdge::Bottom => self.y < 0.0,
            ScreenEdge::Top => self.y > 1.0,
        };
        self.x = self.x.clamp(0.0, 1.0);
        self.y = self.y.clamp(0.0, 1.0);
        if back {
            return Step::Back(match self.edge {
                ScreenEdge::Left | ScreenEdge::Right => self.y,
                ScreenEdge::Top | ScreenEdge::Bottom => self.x,
            });
        }
        Step::There(self.x as f32, self.y as f32)
    }
}

/// The pointer's state, shared by the capture and the event loop.
#[derive(Default)]
struct Flow {
    away: Option<Away>,
    /// Keys (DOM codes) held down over there.
    keys: HashSet<&'static str>,
    /// Buttons held down over there.
    buttons: Vec<MouseButton>,
}

static FLOW: LazyLock<Mutex<Flow>> = LazyLock::new(Default::default);

fn flow() -> std::sync::MutexGuard<'static, Flow> {
    FLOW.lock().unwrap_or_else(|e| e.into_inner())
}

/// The computer that has the pointer, if it's not here.
pub fn away_at() -> Option<DeviceId> {
    flow().away.map(|a| a.peer)
}

// ── Sending ─────────────────────────────────────────────────────────────────

enum Message {
    Pointer(DeviceId, f32, f32),
    Input(DeviceId, RemoteInput),
}

fn send(message: Message) {
    static QUEUE: std::sync::OnceLock<tokio::sync::mpsc::UnboundedSender<Message>> =
        std::sync::OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (queue, mut messages) = tokio::sync::mpsc::unbounded_channel::<Message>();
        core_host::spawn(async move {
            while let Some(message) = messages.recv().await {
                let Some(node) = core_host::node() else { continue };
                let (peer, result) = match message {
                    Message::Pointer(peer, x, y) => (peer, node.remote_abs(peer, x, y).await),
                    Message::Input(peer, input) => (peer, node.remote_input(peer, input).await),
                };
                match result {
                    Ok(()) => {}
                    Err(Error::Denied) => refused(peer, &node).await,
                    Err(Error::Unsupported) => {
                        cool_down(peer);
                        come_back(None);
                        crate::bridge::app::show_message(crate::i18n::tr!(
                            "Mesh",
                            "That computer can't take this mouse and keyboard yet. Update Noddy there."
                        ));
                    }
                    Err(e) => tracing::debug!(error = %e, "Mesh input didn't reach the computer"),
                }
            }
        });
        queue
    });
    let _ = queue.send(message);
}

/// The other computer doesn't allow control from this one: brings the pointer
/// back and asks it to (it shows its prompt).
async fn refused(peer: DeviceId, node: &noddy_core::Node) {
    let already = cooling_down(&peer);
    cool_down(peer);
    if away_at() == Some(peer) {
        come_back(None);
    }
    if !already {
        crate::bridge::app::show_message(crate::i18n::tr!(
            "Mesh",
            "On the other computer, allow remote control from this one in Noddy, then try again."
        ));
        let _ = node.remote_check(peer).await;
    }
}

// ── Moving between computers ────────────────────────────────────────────────

/// Hands the pointer to `peer` through `edge` of this screen, at `along`.
/// `width` × `height` is this screen's size in pixels.
fn go(peer: DeviceId, edge: ScreenEdge, along: f64, width: f64, height: f64) {
    let away = Away::enter(peer, edge, along, width, height);
    {
        let mut flow = flow();
        flow.away = Some(away);
        flow.keys.clear();
        flow.buttons.clear();
    }
    send(Message::Pointer(peer, away.x as f32, away.y as f32));
    input_share::enter_remote(peer);
}

/// Brings the pointer back here, letting go of whatever is held over there.
/// `along` is where along the edge it came back (`None`: wherever it left).
/// Returns where it came back: the edge and the fraction along it.
fn come_back(along: Option<f64>) -> Option<(ScreenEdge, f64)> {
    let (away, keys, buttons) = {
        let mut flow = flow();
        let away = flow.away.take()?;
        (away, std::mem::take(&mut flow.keys), std::mem::take(&mut flow.buttons))
    };
    for code in keys {
        send(Message::Input(away.peer, RemoteInput::KeyEv { code: code.to_owned(), down: false }));
    }
    for button in buttons {
        send(Message::Input(away.peer, RemoteInput::Button { button, action: ButtonAction::Up }));
    }
    input_share::leave_remote();
    let along = along.unwrap_or(match away.edge {
        ScreenEdge::Left | ScreenEdge::Right => away.y,
        ScreenEdge::Top | ScreenEdge::Bottom => away.x,
    });
    #[cfg(windows)]
    hook::place_pointer(away.edge, along);
    Some((away.edge, along))
}

// ── Starting and stopping ───────────────────────────────────────────────────

/// Starts watching the screen edges (Mesh was turned on). Safe to call again.
pub fn start_monitoring() {
    if RUNNING.swap(true, Ordering::AcqRel) {
        return;
    }
    #[cfg(windows)]
    hook::start();
    #[cfg(not(windows))]
    tracing::info!("Mesh can't take this computer's mouse and keyboard on this system yet");
}

/// Stops watching the edges and brings the pointer back (Mesh was turned off).
pub fn stop_monitoring() {
    if !RUNNING.swap(false, Ordering::AcqRel) {
        return;
    }
    come_back(None);
    #[cfg(windows)]
    hook::stop();
}

/// A computer that has the pointer going offline or being removed brings it back.
pub fn on_event(event: &NodeEvent) {
    let gone = match event {
        NodeEvent::DeviceRemoved(id) => Some(*id),
        NodeEvent::LinkChanged { device, link: noddy_core::LinkState::Offline { .. } } => Some(*device),
        _ => None,
    };
    if let Some(id) = gone
        && away_at() == Some(id)
    {
        come_back(None);
    }
}

// ── Keys ────────────────────────────────────────────────────────────────────

/// The DOM `KeyboardEvent.code` of a Windows virtual key as a low-level
/// keyboard hook reports it (left and right modifiers apart), and whether the
/// key is "extended" (Enter on the number pad, the arrow and edit clusters).
/// The inverse of `win::desktop_input::dom_code_to_vk`.
fn vk_to_dom(vk: u32, extended: bool) -> Option<&'static str> {
    const LETTERS: [&str; 26] = [
        "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ", "KeyK", "KeyL",
        "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT", "KeyU", "KeyV", "KeyW", "KeyX",
        "KeyY", "KeyZ",
    ];
    const DIGITS: [&str; 10] =
        ["Digit0", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9"];
    const PAD: [&str; 10] = [
        "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8",
        "Numpad9",
    ];
    const F: [&str; 24] = [
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15",
        "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
    ];
    Some(match vk {
        0x41..=0x5A => LETTERS[(vk - 0x41) as usize],
        0x30..=0x39 => DIGITS[(vk - 0x30) as usize],
        0x60..=0x69 => PAD[(vk - 0x60) as usize],
        0x70..=0x87 => F[(vk - 0x70) as usize],
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D if extended => "NumpadEnter",
        0x0D => "Enter",
        0x13 => "Pause",
        0x14 => "CapsLock",
        0x1B => "Escape",
        0x20 => "Space",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "ArrowLeft",
        0x26 => "ArrowUp",
        0x27 => "ArrowRight",
        0x28 => "ArrowDown",
        0x2C => "PrintScreen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x5B => "MetaLeft",
        0x5C => "MetaRight",
        0x5D => "ContextMenu",
        0x6A => "NumpadMultiply",
        0x6B => "NumpadAdd",
        0x6D => "NumpadSubtract",
        0x6E => "NumpadDecimal",
        0x6F => "NumpadDivide",
        0x90 => "NumLock",
        0x91 => "ScrollLock",
        0xA0 => "ShiftLeft",
        0xA1 => "ShiftRight",
        0xA2 => "ControlLeft",
        0xA3 => "ControlRight",
        0xA4 => "AltLeft",
        0xA5 => "AltRight",
        0xBA => "Semicolon",
        0xBB => "Equal",
        0xBC => "Comma",
        0xBD => "Minus",
        0xBE => "Period",
        0xBF => "Slash",
        0xC0 => "Backquote",
        0xDB => "BracketLeft",
        0xDC => "Backslash",
        0xDD => "BracketRight",
        0xDE => "Quote",
        0xE2 => "IntlBackslash",
        _ => return None,
    })
}

// ── Capturing this computer's mouse and keyboard (Windows) ──────────────────

#[cfg(windows)]
#[allow(unsafe_code)]
mod hook {
    //! Low-level mouse and keyboard hooks on their own thread. While the
    //! pointer is here they only watch for an edge; while it's away they take
    //! every event (so nothing happens here) and send it over there.

    use std::sync::atomic::{AtomicU32, Ordering};

    use noddy_core::{ButtonAction, MouseButton, RemoteInput};
    use windows::Win32::{
        Foundation::{LPARAM, LRESULT, POINT, WPARAM},
        System::Threading::GetCurrentThreadId,
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LBUTTON, VK_MENU},
            WindowsAndMessaging::{
                CallNextHookEx, DispatchMessageW, GUI_INMOVESIZE, GUITHREADINFO, GetGUIThreadInfo,
                GetMessageW, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLMHF_INJECTED, MSG,
                MSLLHOOKSTRUCT, PostThreadMessageW, SetCursorPos, SetWindowsHookExW, TranslateMessage,
                UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
                WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT,
                WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
            },
        },
    };

    use super::{Message, Step, flow, go, send};
    use crate::{input_share::ScreenEdge, win::desktop_input};

    /// The hook thread, to stop it (0 while none runs).
    static THREAD: AtomicU32 = AtomicU32::new(0);
    /// The M key, for Ctrl+Alt+M.
    const VK_M: u32 = 0x4D;
    /// The V key, for Ctrl+Alt+V.
    const VK_V: u32 = 0x56;
    /// Windows already offered during the current drag, so each goes once.
    static OFFERED: std::sync::Mutex<Vec<isize>> = std::sync::Mutex::new(Vec::new());
    /// The most wheel notches sent at once.
    const MAX_NOTCHES: f32 = 50.0;

    /// The window the user is moving or resizing by its title bar or border
    /// right now, with its title, if it's one other computers can be shown.
    fn dragged_window() -> Option<(isize, String)> {
        let mut info =
            GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        // SAFETY: `info` is sized as the call requires; 0 means the foreground thread.
        unsafe { GetGUIThreadInfo(0, &mut info) }.ok()?;
        if info.flags.0 & GUI_INMOVESIZE.0 == 0 {
            return None;
        }
        let hwnd = info.hwndMoveSize.0 as isize;
        crate::win::window_capture::WindowCapture::list_windows().into_iter().find(|(h, _)| *h == hwnd)
    }

    /// Dragging a window against an edge with a computer there offers it to
    /// that computer. True when a window is being dragged (the pointer then
    /// stays with it).
    fn offer_dragged(edge: ScreenEdge) -> bool {
        if !held(VK_LBUTTON) {
            return false;
        }
        let Some((hwnd, title)) = dragged_window() else { return false };
        let fresh = {
            let mut offered = OFFERED.lock().unwrap_or_else(|e| e.into_inner());
            if offered.contains(&hwnd) {
                false
            } else {
                offered.push(hwnd);
                true
            }
        };
        if fresh && let Some(peer) = super::target_at(edge) {
            super::offer_window(peer, hwnd, title);
        }
        true
    }

    pub fn start() {
        std::thread::Builder::new().name("mesh-hooks".into()).spawn(run).ok();
    }

    pub fn stop() {
        let thread = THREAD.swap(0, Ordering::AcqRel);
        if thread != 0 {
            // SAFETY: posting to a thread ID; a thread that is gone just fails.
            let _ = unsafe { PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
    }

    fn run() {
        // SAFETY: the hooks are set and removed on this thread, which pumps
        // messages for them until WM_QUIT.
        unsafe {
            THREAD.store(GetCurrentThreadId(), Ordering::Release);
            let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0);
            let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0);
            if mouse.is_err() || keyboard.is_err() {
                tracing::warn!("Mesh couldn't watch the mouse and keyboard");
            }
            // Mesh may have been turned off before the thread got here.
            if super::RUNNING.load(Ordering::Acquire) {
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            for hook in [mouse, keyboard].into_iter().flatten() {
                let _ = UnhookWindowsHookEx(hook);
            }
            let _ = THREAD.compare_exchange(GetCurrentThreadId(), 0, Ordering::AcqRel, Ordering::Relaxed);
        }
    }

    /// The whole desktop (every monitor), in pixels.
    fn desk() -> desktop_input::ScreenRect {
        desktop_input::virtual_desktop()
    }

    /// The middle of the primary monitor, where the real pointer waits while
    /// it's away.
    fn park_point() -> POINT {
        let m = desktop_input::primary_monitor();
        POINT { x: m.left + m.width / 2, y: m.top + m.height / 2 }
    }

    /// Puts the real pointer just inside `edge`, `along` the way along it.
    pub fn place_pointer(edge: ScreenEdge, along: f64) {
        let d = desk();
        let along = along.clamp(0.0, 1.0);
        let at = |start: i32, len: i32| start + (along * f64::from((len - 1).max(0))).round() as i32;
        let (x, y) = match edge {
            ScreenEdge::Right => (d.left + d.width - 3, at(d.top, d.height)),
            ScreenEdge::Left => (d.left + 2, at(d.top, d.height)),
            ScreenEdge::Bottom => (at(d.left, d.width), d.top + d.height - 3),
            ScreenEdge::Top => (at(d.left, d.width), d.top + 2),
        };
        // SAFETY: plain call.
        let _ = unsafe { SetCursorPos(x, y) };
    }

    /// The edge the pointer at `p` is pushing against, and how far along it.
    fn edge_at(p: POINT) -> Option<(ScreenEdge, f64)> {
        let d = desk();
        if d.width <= 1 || d.height <= 1 {
            return None;
        }
        let fx = f64::from(p.x - d.left) / f64::from(d.width - 1);
        let fy = f64::from(p.y - d.top) / f64::from(d.height - 1);
        if p.x <= d.left {
            Some((ScreenEdge::Left, fy))
        } else if p.x >= d.left + d.width - 1 {
            Some((ScreenEdge::Right, fy))
        } else if p.y <= d.top {
            Some((ScreenEdge::Top, fx))
        } else if p.y >= d.top + d.height - 1 {
            Some((ScreenEdge::Bottom, fx))
        } else {
            None
        }
    }

    fn pass(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // SAFETY: hands the event on as the hook contract requires.
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    /// Taken: nothing here sees the event.
    const TAKEN: LRESULT = LRESULT(1);

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return pass(code, wparam, lparam);
        }
        // SAFETY: for WH_MOUSE_LL, lparam points at an MSLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        // Our own SetCursorPos, and input other apps (or another computer
        // controlling this one) inject, aren't the user's mouse.
        if info.flags & LLMHF_INJECTED != 0 {
            return pass(code, wparam, lparam);
        }
        let message = wparam.0 as u32;
        let away = flow().away;
        let Some(mut away) = away else {
            if message == WM_LBUTTONUP {
                OFFERED.lock().unwrap_or_else(|e| e.into_inner()).clear();
            }
            if message == WM_MOUSEMOVE
                && let Some((edge, _)) = edge_at(info.pt)
                && offer_dragged(edge)
            {
                // A window is being dragged: it goes over as a window, and the
                // pointer stays with it.
                return pass(code, wparam, lparam);
            }
            if message == WM_MOUSEMOVE
                && let Some((edge, along)) = edge_at(info.pt)
                && let Some(peer) = super::target_at(edge)
            {
                let d = desk();
                go(peer, edge, along, f64::from(d.width), f64::from(d.height));
                let park = park_point();
                // SAFETY: plain call.
                let _ = unsafe { SetCursorPos(park.x, park.y) };
                return TAKEN;
            }
            return pass(code, wparam, lparam);
        };
        let peer = away.peer;
        match message {
            WM_MOUSEMOVE => {
                let park = park_point();
                let (dx, dy) = (info.pt.x - park.x, info.pt.y - park.y);
                if dx == 0 && dy == 0 {
                    return TAKEN;
                }
                match away.moved(f64::from(dx), f64::from(dy)) {
                    Step::There(x, y) => {
                        if let Some(a) = flow().away.as_mut() {
                            *a = away;
                        }
                        send(Message::Pointer(peer, x, y));
                    }
                    Step::Back(along) => {
                        super::come_back(Some(along));
                    }
                }
            }
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDOWN | WM_MBUTTONUP => {
                let (button, down) = match message {
                    WM_LBUTTONDOWN => (MouseButton::Left, true),
                    WM_LBUTTONUP => (MouseButton::Left, false),
                    WM_RBUTTONDOWN => (MouseButton::Right, true),
                    WM_RBUTTONUP => (MouseButton::Right, false),
                    WM_MBUTTONDOWN => (MouseButton::Middle, true),
                    _ => (MouseButton::Middle, false),
                };
                {
                    let mut flow = flow();
                    if down {
                        if !flow.buttons.contains(&button) {
                            flow.buttons.push(button);
                        }
                    } else {
                        flow.buttons.retain(|b| *b != button);
                    }
                }
                let action = if down { ButtonAction::Down } else { ButtonAction::Up };
                send(Message::Input(peer, RemoteInput::Button { button, action }));
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                // The high word is the wheel delta: 120 a notch, positive away
                // from the user (up) or to the right.
                let delta = f32::from((info.mouseData >> 16) as u16 as i16) / 120.0;
                let delta = delta.clamp(-MAX_NOTCHES, MAX_NOTCHES);
                // The protocol's notches are positive down and right.
                let (dx, dy) = if message == WM_MOUSEWHEEL { (0.0, -delta) } else { (delta, 0.0) };
                send(Message::Input(peer, RemoteInput::Scroll { dx, dy }));
            }
            // The side buttons have no protocol button yet: they do nothing.
            _ => {}
        }
        TAKEN
    }

    fn held(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> bool {
        // SAFETY: plain call.
        (unsafe { GetAsyncKeyState(i32::from(vk.0)) } as u16 & 0x8000) != 0
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return pass(code, wparam, lparam);
        }
        // SAFETY: for WH_KEYBOARD_LL, lparam points at a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if info.flags.0 & LLKHF_INJECTED.0 != 0 {
            return pass(code, wparam, lparam);
        }
        let message = wparam.0 as u32;
        let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        // Ctrl+Alt+V sends what is copied to the other computer, from here or there.
        if info.vkCode == VK_V && held(VK_CONTROL) && held(VK_MENU) {
            if down {
                super::send_copied();
            }
            return TAKEN;
        }
        let Some(peer) = flow().away.map(|a| a.peer) else {
            return pass(code, wparam, lparam);
        };
        // Ctrl+Alt+M brings the pointer back from anywhere.
        if down && info.vkCode == VK_M && held(VK_CONTROL) && held(VK_MENU) {
            super::come_back(None);
            return TAKEN;
        }
        let extended = info.flags.0 & LLKHF_EXTENDED.0 != 0;
        let Some(code_name) = super::vk_to_dom(info.vkCode, extended) else {
            return TAKEN;
        };
        {
            let mut flow = flow();
            if down {
                flow.keys.insert(code_name);
            } else {
                flow.keys.remove(code_name);
            }
        }
        send(Message::Input(peer, RemoteInput::KeyEv { code: code_name.to_owned(), down }));
        TAKEN
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id_text(fill: u8) -> String {
        DeviceId([fill; 32]).to_string()
    }

    #[test]
    fn devices_on_an_edge_come_lowest_order_first() {
        let layout = vec![
            MeshDeviceLayout { device_id: id_text(0xbb), edge: MeshEdge::Right, order: 1 },
            MeshDeviceLayout { device_id: id_text(0xaa), edge: MeshEdge::Right, order: 0 },
            MeshDeviceLayout { device_id: id_text(0xcc), edge: MeshEdge::Left, order: 0 },
        ];
        let right = devices_on_edge(&layout, ScreenEdge::Right);
        assert_eq!(right, vec![DeviceId([0xaa; 32]), DeviceId([0xbb; 32])]);
        assert!(devices_on_edge(&layout, ScreenEdge::Top).is_empty());
    }

    #[test]
    fn the_pointer_enters_at_the_matching_edge_and_comes_back_across_it() {
        let peer = DeviceId([1; 32]);
        // Leaving through the right edge, halfway down: it appears at the
        // other screen's left edge, halfway down.
        let mut away = Away::enter(peer, ScreenEdge::Right, 0.5, 1000.0, 500.0);
        assert_eq!((away.x, away.y), (0.0, 0.5));
        let close = |step: Step, x: f32, y: f32| matches!(step, Step::There(sx, sy) if (sx - x).abs() < 1e-6 && (sy - y).abs() < 1e-6);
        assert!(close(away.moved(100.0, 50.0), 0.1, 0.6));
        // Pushing against the far edges stays on that screen.
        assert!(close(away.moved(5000.0, 5000.0), 1.0, 1.0));
        // Back left past where it came in returns it, at that height.
        away.moved(-1000.0, -250.0);
        assert!(matches!(away.moved(-1.0, 0.0), Step::Back(along) if (along - 0.5).abs() < 1e-9));
    }

    #[test]
    fn every_edge_enters_opposite_and_returns_the_same_way() {
        let peer = DeviceId([2; 32]);
        for (edge, back) in [
            (ScreenEdge::Left, (1.0, 0.0)),
            (ScreenEdge::Right, (-1.0, 0.0)),
            (ScreenEdge::Top, (0.0, 1.0)),
            (ScreenEdge::Bottom, (0.0, -1.0)),
        ] {
            let mut away = Away::enter(peer, edge, 0.25, 100.0, 100.0);
            assert!(matches!(away.moved(back.0, back.1), Step::Back(along) if (along - 0.25).abs() < 1e-9));
        }
    }

    #[test]
    fn keys_map_to_dom_codes() {
        assert_eq!(vk_to_dom(0x41, false), Some("KeyA"));
        assert_eq!(vk_to_dom(0x39, false), Some("Digit9"));
        assert_eq!(vk_to_dom(0x65, false), Some("Numpad5"));
        assert_eq!(vk_to_dom(0x87, false), Some("F24"));
        assert_eq!(vk_to_dom(0x0D, false), Some("Enter"));
        assert_eq!(vk_to_dom(0x0D, true), Some("NumpadEnter"));
        assert_eq!(vk_to_dom(0xA3, true), Some("ControlRight"));
        assert_eq!(vk_to_dom(0xFF, false), None);
    }

    /// Every key the other computer understands comes back as the same key.
    #[cfg(windows)]
    #[test]
    fn keys_round_trip_with_the_receiving_side() {
        for vk in 0u32..=0xFF {
            for extended in [false, true] {
                if let Some(code) = vk_to_dom(vk, extended) {
                    let (back, _) = crate::win::desktop_input::dom_code_to_vk(code).unwrap();
                    assert_eq!(u32::from(back), vk, "{code}");
                }
            }
        }
    }

    #[test]
    fn the_hotkey_sends_to_the_lowest_order_computer_then_the_first_edge() {
        let layout = vec![
            MeshDeviceLayout { device_id: id_text(3), edge: MeshEdge::Top, order: 1 },
            MeshDeviceLayout { device_id: id_text(2), edge: MeshEdge::Right, order: 0 },
            MeshDeviceLayout { device_id: id_text(1), edge: MeshEdge::Left, order: 0 },
        ];
        // Order 0 on the Left comes before order 0 on the Right.
        assert_eq!(default_target(&layout), Some(DeviceId([1; 32])));
        assert_eq!(default_target(&layout[..2]), Some(DeviceId([2; 32])));
        assert_eq!(default_target(&[]), None);
    }

    #[test]
    fn a_dragged_window_is_offered_by_its_app_window_id() {
        let offer = noddy_core::WindowOffer { key: window_key(264_526), title: "Notes".into() };
        assert_eq!(offer.key, "win:264526");
        assert!(offer.is_valid());
    }

    #[test]
    fn layout_updates_are_kept() {
        let layout = vec![MeshDeviceLayout { device_id: id_text(0xaa), edge: MeshEdge::Left, order: 0 }];
        update_layout(layout.clone());
        assert_eq!(*LAYOUT.lock().unwrap(), layout);
    }
}
