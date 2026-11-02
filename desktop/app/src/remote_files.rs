// SPDX-License-Identifier: GPL-3.0-or-later
//! Browsing the folders another paired computer shares: one window of it at a
//! time (the computer, the folder shown, its entries), and downloading a file
//! from it into `Downloads\Noddy`. What the other computer shares is its own
//! decision (core refuses until it allows this computer); this only shows what
//! it says.

use std::sync::{Mutex, MutexGuard};

use noddy_core::{DeviceId, DeviceKind, Error, NodeEvent, StorageEntry};

use crate::{
    bridge::app::{describe, show_message},
    core_host,
    i18n::tr,
    remote_paths,
    state::Changes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// Nothing open.
    #[default]
    Idle,
    Loading,
    Ready,
    /// The computer doesn't share its folders with this one (yet).
    NotShared,
    Failed,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Loading => "loading",
            Phase::Ready => "ready",
            Phase::NotShared => "not_shared",
            Phase::Failed => "failed",
        }
    }
}

#[derive(Default)]
struct State {
    device: Option<DeviceId>,
    device_name: String,
    path: String,
    phase: Phase,
    entries: Vec<StorageEntry>,
    message: String,
    /// The file being downloaded, by its path on the other computer.
    downloading: Option<String>,
    /// How much of it is saved, out of its size (0 until it is known).
    downloaded: u64,
    download_size: u64,
    /// A computer that asked to browse this one's folders and isn't allowed.
    request: Option<DeviceId>,
    /// Bumped by every listing, so a slow answer for a folder the user already left is dropped.
    generation: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut guard: MutexGuard<'_, Option<State>> = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default))
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub device: String,
    pub device_name: String,
    pub path: String,
    pub phase: &'static str,
    pub entries_json: String,
    pub crumbs_json: String,
    pub message: String,
    pub downloading: String,
    /// How far the download is, `0.0..=1.0` (0 until the size is known).
    pub progress: f64,
    pub can_go_up: bool,
    /// A computer asking to browse this one's folders: its ID and name.
    pub request_device: String,
    pub request_name: String,
}

pub fn view() -> View {
    let mut view = state(|s| View {
        device: s.device.map(|d| d.to_string()).unwrap_or_default(),
        device_name: s.device_name.clone(),
        path: s.path.clone(),
        phase: s.phase.as_str(),
        entries_json: remote_paths::entries_json(&s.path, &s.entries),
        crumbs_json: remote_paths::crumbs_json(&s.device_name, &s.path),
        message: s.message.clone(),
        downloading: s.downloading.clone().unwrap_or_default(),
        progress: if s.download_size == 0 {
            0.0
        } else {
            (s.downloaded as f64 / s.download_size as f64).clamp(0.0, 1.0)
        },
        can_go_up: !s.path.is_empty(),
        request_device: s.request.map(|d| d.to_string()).unwrap_or_default(),
        request_name: String::new(),
    });
    // Looked up outside the state's lock: the hub has its own.
    if let Ok(id) = view.request_device.parse::<DeviceId>() {
        view.request_name = core_host::host().hub.read(|h| h.name_of(&id)).unwrap_or_else(|| id.short());
    }
    view
}

fn changed() {
    core_host::host().hub.changed(Changes::REMOTE_FILES);
}

/// Starts browsing `device`'s shared folders from the top.
pub fn open(device: DeviceId) {
    let name = core_host::host().hub.read(|s| s.name_of(&device)).unwrap_or_else(|| "computer".into());
    state(|s| {
        // The count keeps rising so an answer to an earlier browse is never mistaken for this one's.
        *s = State {
            device: Some(device),
            device_name: name,
            generation: s.generation,
            request: s.request,
            ..State::default()
        };
    });
    list("");
}

/// Shows the folder `path` (relative to the top) of the computer being browsed.
pub fn list(path: &str) {
    let (device, generation) = state(|s| {
        s.generation += 1;
        s.path = path.to_owned();
        s.phase = Phase::Loading;
        s.message.clear();
        s.entries.clear();
        (s.device, s.generation)
    });
    changed();
    let (Some(device), Some(node)) = (device, core_host::node()) else { return };
    let path = path.to_owned();
    core_host::spawn(async move {
        let result = node.storage_list(device, path.clone()).await;
        let applied = state(|s| {
            if s.generation != generation {
                return false; // the user has gone elsewhere since
            }
            match result {
                Ok(mut entries) => {
                    remote_paths::sort_entries(&mut entries);
                    s.entries = entries;
                    s.phase = Phase::Ready;
                    s.message.clear();
                }
                Err(Error::Denied) => {
                    s.phase = Phase::NotShared;
                    s.message = tr!(
                        "RemoteFiles",
                        "%1 hasn't let this computer browse its folders. On that computer, open Settings › Devices, pick this computer and turn on \"Share folders\".",
                        s.device_name
                    );
                }
                Err(e) => {
                    s.phase = Phase::Failed;
                    s.message = describe(&e);
                }
            }
            true
        });
        if applied {
            changed();
        }
    });
}

/// Goes up one folder.
pub fn up() {
    if let Some(parent) = remote_paths::parent(&state(|s| s.path.clone())) {
        list(&parent);
    }
}

/// Reloads the folder shown.
pub fn refresh() {
    let path = state(|s| s.path.clone());
    list(&path);
}

/// Stops browsing.
pub fn close() {
    state(|s| {
        s.generation += 1;
        *s = State { generation: s.generation, request: s.request, ..State::default() };
    });
    changed();
}

/// Downloads the file at `remote` (a path from the listing) into `Downloads\Noddy`.
/// It carries on from a part left by an earlier try.
pub fn download(remote: &str) {
    let Some((device, name)) = state(|s| {
        if s.downloading.is_some() {
            return None; // one at a time
        }
        s.downloading = Some(remote.to_owned());
        s.downloaded = 0;
        s.download_size = 0;
        s.device.map(|d| (d, s.device_name.clone()))
    }) else {
        return;
    };
    changed();
    let Some(node) = core_host::node() else {
        state(|s| s.downloading = None);
        changed();
        return;
    };
    let remote = remote.to_owned();
    core_host::spawn(async move {
        let file_name = remote_paths::safe_file_name(remote_paths::leaf(&remote));
        let dir = core_host::downloads_dir();
        let dest = crate::photos::free_path(&dir, &file_name);
        // Told about every chunk, shown at most ten times a second (and at the end).
        let mut last: Option<std::time::Instant> = None;
        let result = node
            .storage_download_with_progress(device, remote.clone(), &dest, |done, total| {
                let finished = done >= total;
                if !finished && last.is_some_and(|at| at.elapsed() < std::time::Duration::from_millis(100)) {
                    return;
                }
                last = Some(std::time::Instant::now());
                state(|s| {
                    s.downloaded = done;
                    s.download_size = total;
                });
                changed();
            })
            .await;
        state(|s| {
            s.downloading = None;
            s.downloaded = 0;
            s.download_size = 0;
        });
        changed();
        match result {
            Ok(_) => {
                let saved = dest.file_name().and_then(|n| n.to_str()).unwrap_or(&file_name).to_owned();
                if cfg!(windows) {
                    show_message(tr!("RemoteFiles", "Saved %1 from %2 to Downloads\\Noddy.", saved, name));
                } else {
                    show_message(tr!("RemoteFiles", "Saved %1 from %2 to Downloads/Noddy.", saved, name));
                }
            }
            Err(Error::Denied) => {
                show_message(tr!("RemoteFiles", "%1 didn't allow that download.", name));
            }
            Err(e) => {
                tracing::warn!(error = %e, "couldn't download from another computer");
                show_message(describe(&e));
            }
        }
    });
}

/// Notices a computer asking to browse this one's folders before it is allowed,
/// so the user can say yes (phones are asked about in their own way).
pub fn on_event(event: &NodeEvent) {
    match event {
        NodeEvent::StorageRequested { device } => {
            let computer = core_host::host().hub.read(|h| {
                h.device(device).map(|d| matches!(d.info.kind, DeviceKind::Desktop | DeviceKind::Laptop))
            });
            if computer == Some(true) {
                state(|s| s.request = Some(*device));
                changed();
            }
        }
        NodeEvent::DeviceRemoved(device) => {
            let cleared = state(|s| {
                if s.request == Some(*device) {
                    s.request = None;
                    true
                } else {
                    false
                }
            });
            if cleared {
                changed();
            }
        }
        _ => {}
    }
}

/// Lets `device` browse this computer's shared folders (the answer to the request).
pub fn allow_request(device: DeviceId) {
    if let Some(node) = core_host::node()
        && let Err(e) = node.set_device_toggle(device, "share_folders", true)
    {
        show_message(describe(&e));
    }
    dismiss_request();
}

/// Leaves the request unanswered; the other computer is told how to ask again.
pub fn dismiss_request() {
    state(|s| s.request = None);
    changed();
}
