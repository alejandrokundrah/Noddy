// SPDX-License-Identifier: MPL-2.0
//! The unified device list: every device this one knows about or can see, in
//! one list, with where it stands and what can be done with it.
//!
//! A device is in one state at a time:
//!
//! | State | Meaning |
//! |---|---|
//! | [`Discovered`](DeviceState::Discovered) | Seen on the network, not paired. Nothing of it is reachable. |
//! | [`Pairing`](DeviceState::Pairing) | A pairing with it is in progress. |
//! | [`Paired`](DeviceState::Paired) | Trusted, and connecting. |
//! | [`Connected`](DeviceState::Connected) | Trusted, with a live session. |
//! | [`Offline`](DeviceState::Offline) | Trusted, but not reachable now. |
//!
//! Discovery never grants access: an entry that is merely discovered has no
//! actions.

use std::collections::BTreeSet;

use noddy_protocol::{DeviceId, messages::DeviceKind};

use crate::{ConnectionPath, DiscoveredDevice, LinkState, PairedDevice, storage::STORAGE_READ};

/// Where a device stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Discovered,
    Pairing,
    Paired,
    Connected,
    Offline,
}

impl DeviceState {
    /// Lower sorts first: what can be used now comes first.
    fn rank(self) -> u8 {
        match self {
            DeviceState::Connected => 0,
            DeviceState::Paired => 1,
            DeviceState::Offline => 2,
            DeviceState::Pairing => 3,
            DeviceState::Discovered => 4,
        }
    }
}

/// Something that can be done with a device right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    /// Browse the folders it shares, and download from them. The device may
    /// still say no: what it shares is its own decision.
    BrowseFiles,
}

/// One device in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    pub id: DeviceId,
    /// Its name, or its short ID when it hasn't announced one.
    pub name: String,
    /// [`DeviceKind::Unknown`] until paired: a device nearby doesn't say what it is.
    pub kind: DeviceKind,
    pub os: Option<String>,
    pub state: DeviceState,
    /// How the live session is carried (only when connected).
    pub path: Option<ConnectionPath>,
    pub actions: Vec<DeviceAction>,
}

/// A paired device with the capabilities it announced.
pub(crate) struct KnownDevice {
    pub device: PairedDevice,
    pub caps: BTreeSet<String>,
}

/// Puts what is known into one list: paired devices (by their link), a pairing
/// in progress, and devices seen nearby that aren't paired.
pub(crate) fn assemble(
    known: Vec<KnownDevice>,
    discovered: Vec<DiscoveredDevice>,
    pairing: Option<DeviceId>,
) -> Vec<DeviceEntry> {
    let mut entries: Vec<DeviceEntry> = known
        .into_iter()
        .map(|KnownDevice { device, caps }| {
            let (state, path) = match device.link {
                LinkState::Online { path, .. } => (DeviceState::Connected, Some(path)),
                LinkState::Connecting => (DeviceState::Paired, None),
                LinkState::Offline { .. } => (DeviceState::Offline, None),
            };
            let mut actions = Vec::new();
            if state == DeviceState::Connected && caps.contains(STORAGE_READ) {
                actions.push(DeviceAction::BrowseFiles);
            }
            DeviceEntry {
                id: device.id,
                name: device.info.name.clone(),
                kind: device.info.kind,
                os: Some(device.info.os.clone()),
                state,
                path,
                actions,
            }
        })
        .collect();

    for seen in discovered {
        // A paired device is already listed; a device seen again is listed once.
        if entries.iter().any(|e| e.id == seen.id) {
            continue;
        }
        let state = if pairing == Some(seen.id) { DeviceState::Pairing } else { DeviceState::Discovered };
        entries.push(DeviceEntry {
            id: seen.id,
            name: seen.name.filter(|n| !n.is_empty()).unwrap_or_else(|| seen.id.short()),
            kind: DeviceKind::Unknown,
            os: None,
            state,
            path: None,
            actions: Vec::new(),
        });
    }
    // The device being paired with may not have been seen on the network (a
    // link was scanned, or the address typed in).
    if let Some(id) = pairing
        && !entries.iter().any(|e| e.id == id)
    {
        entries.push(DeviceEntry {
            id,
            name: id.short(),
            kind: DeviceKind::Unknown,
            os: None,
            state: DeviceState::Pairing,
            path: None,
            actions: Vec::new(),
        });
    }

    entries.sort_by(|a, b| {
        a.state
            .rank()
            .cmp(&b.state.rank())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    entries
}

#[cfg(test)]
mod tests {
    use noddy_protocol::messages::DeviceInfo;

    use super::*;

    fn info(name: &str, kind: DeviceKind, os: &str) -> DeviceInfo {
        DeviceInfo {
            name: name.into(),
            kind,
            os: os.into(),
            os_ver: String::new(),
            model: None,
            accent: None,
            screen: None,
            wallpaper: None,
            install_id: None,
        }
    }

    fn paired(n: u8, name: &str, kind: DeviceKind, link: LinkState) -> PairedDevice {
        PairedDevice {
            id: DeviceId([n; 32]),
            info: info(name, kind, if kind == DeviceKind::Phone { "android" } else { "windows" }),
            paired_at: 0,
            link,
            can_wake: false,
            reconnect: true,
        }
    }

    fn online() -> LinkState {
        LinkState::Online { path: ConnectionPath::Lan, rtt_ms: 3 }
    }

    fn known(device: PairedDevice, caps: &[&str]) -> KnownDevice {
        KnownDevice { device, caps: caps.iter().map(|c| (*c).to_owned()).collect() }
    }

    fn seen(n: u8, name: Option<&str>) -> DiscoveredDevice {
        DiscoveredDevice { id: DeviceId([n; 32]), name: name.map(str::to_owned) }
    }

    #[test]
    fn every_state_has_its_place_and_usable_devices_come_first() {
        let list = assemble(
            vec![
                known(
                    paired(1, "Offline PC", DeviceKind::Desktop, LinkState::Offline { last_seen: Some(5) }),
                    &[],
                ),
                known(paired(2, "Laptop", DeviceKind::Laptop, online()), &[]),
                known(paired(3, "Dialing PC", DeviceKind::Desktop, LinkState::Connecting), &[]),
            ],
            vec![seen(4, Some("Stranger")), seen(5, Some("Joiner"))],
            Some(DeviceId([5; 32])),
        );
        let states: Vec<_> = list.iter().map(|e| (e.name.as_str(), e.state)).collect();
        assert_eq!(
            states,
            [
                ("Laptop", DeviceState::Connected),
                ("Dialing PC", DeviceState::Paired),
                ("Offline PC", DeviceState::Offline),
                ("Joiner", DeviceState::Pairing),
                ("Stranger", DeviceState::Discovered),
            ]
        );
    }

    #[test]
    fn a_device_that_is_only_discovered_offers_nothing() {
        let list = assemble(Vec::new(), vec![seen(9, Some("Stranger"))], None);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, DeviceKind::Unknown);
        assert_eq!(list[0].os, None);
        assert_eq!(list[0].path, None);
        assert!(list[0].actions.is_empty(), "discovery doesn't grant access");
    }

    #[test]
    fn browsing_is_offered_only_when_connected_and_the_device_offers_folders() {
        let list = assemble(
            vec![
                known(paired(1, "Sharing PC", DeviceKind::Desktop, online()), &["storage.read"]),
                known(paired(2, "Quiet PC", DeviceKind::Desktop, online()), &["media.control"]),
                known(
                    paired(3, "Sleeping PC", DeviceKind::Desktop, LinkState::Offline { last_seen: None }),
                    &["storage.read"],
                ),
            ],
            Vec::new(),
            None,
        );
        let actions: Vec<_> = list.iter().map(|e| (e.name.as_str(), e.actions.clone())).collect();
        assert_eq!(
            actions,
            [("Quiet PC", vec![]), ("Sharing PC", vec![DeviceAction::BrowseFiles]), ("Sleeping PC", vec![]),]
        );
        assert_eq!(list[0].path, Some(ConnectionPath::Lan));
        assert_eq!(list[2].path, None, "an offline device has no path");
    }

    #[test]
    fn a_paired_device_seen_on_the_network_is_listed_once() {
        let list = assemble(
            vec![known(paired(1, "Laptop", DeviceKind::Laptop, online()), &[])],
            vec![seen(1, Some("Laptop")), seen(1, Some("Laptop"))],
            None,
        );
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].state, DeviceState::Connected);
    }

    #[test]
    fn a_pairing_with_an_unseen_device_still_shows_up() {
        let id = DeviceId([7; 32]);
        let list = assemble(Vec::new(), Vec::new(), Some(id));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].state, DeviceState::Pairing);
        assert_eq!(list[0].name, id.short(), "no name known yet: its short ID");
    }

    #[test]
    fn an_empty_announced_name_falls_back_to_the_short_id() {
        let list = assemble(Vec::new(), vec![seen(3, Some("")), seen(4, None)], None);
        assert!(list.iter().all(|e| e.name == e.id.short()));
    }

    #[test]
    fn devices_in_one_state_are_sorted_by_name_ignoring_case() {
        let list = assemble(
            vec![
                known(paired(1, "beta", DeviceKind::Desktop, online()), &[]),
                known(paired(2, "Alpha", DeviceKind::Desktop, online()), &[]),
                known(paired(3, "Gamma", DeviceKind::Desktop, online()), &[]),
            ],
            Vec::new(),
            None,
        );
        assert_eq!(list.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Alpha", "beta", "Gamma"]);
    }
}
