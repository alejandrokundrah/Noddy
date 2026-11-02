// SPDX-License-Identifier: GPL-3.0-or-later
//! `RemoteFiles`: QML singleton for browsing the folders another paired
//! computer shares (what the window shows, and its actions).

use std::pin::Pin;

use cxx_qt::Threading;
use cxx_qt_lib::QString;

use crate::{remote_files, state::Changes};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, device)]
        #[qproperty(QString, device_name, cxx_name = "deviceName")]
        #[qproperty(QString, path)]
        #[qproperty(QString, phase)]
        #[qproperty(QString, entries)]
        #[qproperty(QString, crumbs)]
        #[qproperty(QString, message)]
        #[qproperty(QString, downloading)]
        #[qproperty(f64, progress)]
        #[qproperty(QString, request_device, cxx_name = "requestDevice")]
        #[qproperty(QString, request_name, cxx_name = "requestName")]
        #[qproperty(bool, can_go_up, cxx_name = "canGoUp")]
        type RemoteFiles = super::RemoteFilesRust;
    }

    impl cxx_qt::Threading for RemoteFiles {}
    impl cxx_qt::Initialize for RemoteFiles {}

    unsafe extern "RustQt" {
        /// Starts browsing a computer's shared folders from the top.
        #[qinvokable]
        fn open(self: &RemoteFiles, device: &QString);
        /// Shows a folder (a path from `entries`, or "" for the top).
        #[qinvokable]
        fn list(self: &RemoteFiles, path: &QString);
        #[qinvokable]
        fn up(self: &RemoteFiles);
        #[qinvokable]
        fn refresh(self: &RemoteFiles);
        #[qinvokable]
        fn close(self: &RemoteFiles);
        /// Downloads a file (a path from `entries`) into Downloads\Noddy.
        #[qinvokable]
        fn download(self: &RemoteFiles, path: &QString);
        /// Allows the computer that asked to browse this one's folders.
        #[qinvokable]
        #[cxx_name = "allowRequest"]
        fn allow_request(self: &RemoteFiles, device: &QString);
        #[qinvokable]
        #[cxx_name = "dismissRequest"]
        fn dismiss_request(self: &RemoteFiles);
    }
}

#[derive(Default)]
pub struct RemoteFilesRust {
    device: QString,
    device_name: QString,
    path: QString,
    phase: QString,
    entries: QString,
    crumbs: QString,
    message: QString,
    downloading: QString,
    progress: f64,
    request_device: QString,
    request_name: QString,
    can_go_up: bool,
}

impl cxx_qt::Initialize for qobject::RemoteFiles {
    fn initialize(self: Pin<&mut Self>) {
        super::subscribe(self.qt_thread(), Changes::REMOTE_FILES, Self::sync);
    }
}

impl qobject::RemoteFiles {
    /// Copies what the window shows into the properties.
    fn sync(mut self: Pin<&mut Self>) {
        let v = remote_files::view();
        self.as_mut().set_device(QString::from(&v.device));
        self.as_mut().set_device_name(QString::from(&v.device_name));
        self.as_mut().set_path(QString::from(&v.path));
        self.as_mut().set_phase(QString::from(v.phase));
        self.as_mut().set_entries(QString::from(&v.entries_json));
        self.as_mut().set_crumbs(QString::from(&v.crumbs_json));
        self.as_mut().set_message(QString::from(&v.message));
        self.as_mut().set_downloading(QString::from(&v.downloading));
        self.as_mut().set_progress(v.progress);
        self.as_mut().set_request_device(QString::from(&v.request_device));
        self.as_mut().set_request_name(QString::from(&v.request_name));
        self.as_mut().set_can_go_up(v.can_go_up);
    }

    pub fn open(&self, device: &QString) {
        if let Some(id) = super::parse_device(device) {
            remote_files::open(id);
        }
    }

    pub fn list(&self, path: &QString) {
        remote_files::list(&String::from(path));
    }

    pub fn up(&self) {
        remote_files::up();
    }

    pub fn refresh(&self) {
        remote_files::refresh();
    }

    pub fn close(&self) {
        remote_files::close();
    }

    pub fn download(&self, path: &QString) {
        remote_files::download(&String::from(path));
    }

    pub fn allow_request(&self, device: &QString) {
        if let Some(id) = super::parse_device(device) {
            remote_files::allow_request(id);
        }
    }

    pub fn dismiss_request(&self) {
        remote_files::dismiss_request();
    }
}
