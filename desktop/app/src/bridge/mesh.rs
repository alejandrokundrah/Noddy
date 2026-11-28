// SPDX-License-Identifier: GPL-3.0-or-later
//! `Mesh` Qt singleton: exposes Mesh enable/disable and monitor-layout to QML.

use std::pin::Pin;

use cxx_qt::Threading;
use cxx_qt_lib::QString;

use crate::{
    core_host,
    settings::{self, MeshDeviceLayout, MeshEdge, Settings},
    state::Changes,
};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// Whether the Mesh unified-pointer/clipboard/notification feature is active.
        #[qproperty(bool, enabled)]
        /// JSON array of `{ deviceId, edge, order }` layout entries.
        #[qproperty(QString, layout_json)]
        type Mesh = super::MeshRust;

        /// Enables or disables Mesh (starts/stops the cursor-edge monitor thread).
        #[qinvokable]
        fn set_mesh_enabled(self: Pin<&mut Mesh>, on: bool);

        /// Saves a new monitor layout from a JSON array of `{ deviceId, edge, order }`.
        #[qinvokable]
        fn set_mesh_layout(self: Pin<&mut Mesh>, json: &QString);

        /// Returns a JSON array of all currently paired device IDs (for the monitor map UI).
        #[qinvokable]
        fn paired_devices_json(self: &Mesh) -> QString;
    }

    impl cxx_qt::Threading for Mesh {}
    impl cxx_qt::Initialize for Mesh {}
}

#[derive(Default)]
pub struct MeshRust {
    enabled: bool,
    layout_json: QString,
}

impl cxx_qt::Initialize for qobject::Mesh {
    fn initialize(mut self: Pin<&mut Self>) {
        let enabled = settings::mesh_enabled();
        let layout = settings::mesh_layout();
        self.as_mut().set_enabled(enabled);
        self.as_mut().set_layout_json(QString::from(&layout_to_json(&layout)));

        // Refresh when PREFS changes (e.g. another instance wrote settings).
        super::subscribe(self.qt_thread(), Changes::PREFS, Self::refresh);
    }
}

impl qobject::Mesh {
    fn refresh(mut self: Pin<&mut Self>) {
        let enabled = settings::mesh_enabled();
        if *self.enabled() != enabled {
            self.as_mut().set_enabled(enabled);
        }
        let layout = settings::mesh_layout();
        let json = QString::from(&layout_to_json(&layout));
        if *self.layout_json() != json {
            self.as_mut().set_layout_json(json);
        }
    }

    pub fn set_mesh_enabled(mut self: Pin<&mut Self>, on: bool) {
        settings::set_mesh_enabled(on);
        self.as_mut().set_enabled(on);
        save();
    }

    pub fn set_mesh_layout(mut self: Pin<&mut Self>, json: &QString) {
        let layout = parse_layout_json(&String::from(json));
        settings::set_mesh_layout(layout.clone());
        self.as_mut().set_layout_json(QString::from(&layout_to_json(&layout)));
        save();
    }

    pub fn paired_devices_json(&self) -> QString {
        let items = core_host::host().hub.read(|s| {
            s.devices
                .iter()
                .map(|d| {
                    let name = crate::settings::nickname_for(&d.id.to_string())
                        .unwrap_or_else(|| d.info.name.clone());
                    serde_json::json!({
                        "id": d.id.to_string(),
                        "name": name,
                        "online": matches!(d.link, noddy_core::LinkState::Online { .. }),
                        // Only computers can take the pointer (crate::mesh).
                        "computer": matches!(d.info.kind, noddy_core::DeviceKind::Desktop | noddy_core::DeviceKind::Laptop),
                    })
                })
                .collect::<Vec<_>>()
        });
        QString::from(&serde_json::Value::Array(items).to_string())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn layout_to_json(layout: &[MeshDeviceLayout]) -> String {
    let items: Vec<serde_json::Value> = layout
        .iter()
        .map(|d| {
            serde_json::json!({
                "deviceId": d.device_id,
                "edge": edge_str(d.edge),
                "order": d.order,
            })
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

fn parse_layout_json(json: &str) -> Vec<MeshDeviceLayout> {
    let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json) else {
        return Vec::new();
    };
    arr.into_iter()
        .filter_map(|v| {
            let device_id = v["deviceId"].as_str()?.to_owned();
            let edge = parse_edge(v["edge"].as_str()?)?;
            let order = v["order"].as_u64().unwrap_or(0) as u32;
            Some(MeshDeviceLayout { device_id, edge, order })
        })
        .collect()
}

fn edge_str(edge: MeshEdge) -> &'static str {
    match edge {
        MeshEdge::Left => "left",
        MeshEdge::Right => "right",
        MeshEdge::Top => "top",
        MeshEdge::Bottom => "bottom",
    }
}

fn parse_edge(s: &str) -> Option<MeshEdge> {
    match s {
        "left" => Some(MeshEdge::Left),
        "right" => Some(MeshEdge::Right),
        "top" => Some(MeshEdge::Top),
        "bottom" => Some(MeshEdge::Bottom),
        _ => None,
    }
}

fn save() {
    let enabled = settings::mesh_enabled();
    let layout = settings::mesh_layout();
    let dir = core_host::host().data_dir.clone();
    std::thread::spawn(move || {
        let mut s = Settings::load(&dir);
        s.mesh_enabled = enabled;
        s.mesh_layout = layout;
        let _ = s.save(&dir);
    });
}
