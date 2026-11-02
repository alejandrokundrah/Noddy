// SPDX-License-Identifier: GPL-3.0-or-later
//! Browsing the folders another paired computer shares: one window of it at a
//! time (the computer, the folder shown, its entries), and downloading a file
//! from it into `Downloads\Noddy`. What the other computer shares is its own
//! decision (core refuses until it allows this computer); this only shows what
//! it says.

use std::sync::{Mutex, MutexGuard};

use noddy_core::{DeviceId, Error, StorageEntry};

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
    /// Bumped by every listing, so a slow answer for a folder the user already left is dropped.
    generation: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut guard: MutexGuard<'_, Option<State>> = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default))
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub device: String,
    pub device_name: String,
    pub path: String,
    pub phase: &'static str,
    pub entries_json: String,
    pub crumbs_json: String,
    pub message: String,
    pub downloading: String,
    pub can_go_up: bool,
}

pub fn view() -> View {
    state(|s| View {
        device: s.device.map(|d| d.to_string()).unwrap_or_default(),
        device_name: s.device_name.clone(),
        path: s.path.clone(),
        phase: s.phase.as_str(),
        entries_json: remote_paths::entries_json(&s.path, &s.entries),
        crumbs_json: remote_paths::crumbs_json(&s.device_name, &s.path),
        message: s.message.clone(),
        downloading: s.downloading.clone().unwrap_or_default(),
        can_go_up: !s.path.is_empty(),
    })
}

fn changed() {
    core_host::host().hub.changed(Changes::REMOTE_FILES);
}

/// Starts browsing `device`'s shared folders from the top.
pub fn open(device: DeviceId) {
    let name = core_host::host().hub.read(|s| s.name_of(&device)).unwrap_or_else(|| "computer".into());
    state(|s| {
        // The count keeps rising so an answer to an earlier browse is never mistaken for this one's.
        *s = State { device: Some(device), device_name: name, generation: s.generation, ..State::default() };
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
        *s = State { generation: s.generation, ..State::default() };
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
        let result = node.storage_download(device, remote.clone(), &dest).await;
        state(|s| s.downloading = None);
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
