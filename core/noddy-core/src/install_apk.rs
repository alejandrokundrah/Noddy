// SPDX-License-Identifier: MPL-2.0
//! Install APK files on a USB-connected phone from the PC
//! (`docs/protocol/install-apk.md`).
//!
//! This feature uses the PC's local ADB server to push and install an APK on
//! the phone. It requires:
//!
//! - The phone is connected to the PC over USB.
//! - USB debugging is enabled and authorized on the phone.
//! - The ADB server is running on the PC.
//!
//! The install is entirely local — it does not use the Noddy protocol between
//! the two apps; it goes directly through ADB. Progress and the outcome are
//! reported as [`NodeEvent::ApkInstall`] events.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use noddy_protocol::DeviceId;

use crate::{events::NodeEvent, node::Shared};

/// Progress and outcome of an APK installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApkInstallState {
    /// The APK is being pushed and installed.
    Installing,
    /// Installation succeeded.
    Done { message: String },
    /// Installation failed.
    Failed { reason: String },
}

/// Installs an APK on a USB-connected phone from the PC.
///
/// The phone must be connected over USB with adb debugging enabled, identified
/// by its `DeviceId`. The APK file at `apk_path` is streamed to the device
/// through `cmd package install -S <size>`.
///
/// Progress and the result are emitted as [`NodeEvent::ApkInstall`] events.
pub(crate) async fn install(shared: &Arc<Shared>, peer: DeviceId, apk_path: PathBuf) -> crate::Result<()> {
    // Look up this phone's USB serial and ADB server address.
    let (serial, adb_addr) = {
        let guard = shared.usb_serials.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(&peer).cloned().ok_or(crate::Error::Unsupported)?
    };

    // Validate the APK path.
    let apk_path_clone = apk_path.clone();
    if !apk_path.exists() {
        return Err(crate::Error::Internal("APK file not found".into()));
    }
    let extension = apk_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !extension.eq_ignore_ascii_case("apk") {
        return Err(crate::Error::Internal("file is not an APK (expected .apk extension)".into()));
    }

    // Emit Installing state.
    shared.emit(NodeEvent::ApkInstall {
        device: peer,
        path: apk_path_clone.clone(),
        state: ApkInstallState::Installing,
    });

    // Spawn the actual install in a task so the caller doesn't block.
    let shared = shared.clone();
    tokio::spawn(async move {
        let result = do_install(adb_addr, &serial, &apk_path_clone).await;
        match result {
            Ok(message) => {
                tracing::info!(peer = %peer.short(), "APK installed successfully");
                shared.emit(NodeEvent::ApkInstall {
                    device: peer,
                    path: apk_path_clone,
                    state: ApkInstallState::Done { message },
                });
            }
            Err(reason) => {
                tracing::warn!(peer = %peer.short(), %reason, "APK installation failed");
                shared.emit(NodeEvent::ApkInstall {
                    device: peer,
                    path: apk_path_clone,
                    state: ApkInstallState::Failed { reason: reason.clone() },
                });
            }
        }
    });

    Ok(())
}

/// The actual ADB install call.
async fn do_install(
    adb_addr: SocketAddr,
    serial: &str,
    apk_path: &std::path::Path,
) -> std::result::Result<String, String> {
    noddy_adb::install_apk(adb_addr, serial, apk_path).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apk_install_state_debug() {
        let state = ApkInstallState::Installing;
        assert_eq!(format!("{state:?}"), "Installing");
        let state = ApkInstallState::Done { message: "Success".into() };
        assert!(format!("{state:?}").contains("Success"));
        let state = ApkInstallState::Failed { reason: "INSTALL_FAILED_ALREADY_EXISTS".into() };
        assert!(format!("{state:?}").contains("INSTALL_FAILED"));
    }
}
