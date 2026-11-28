// SPDX-License-Identifier: GPL-3.0-or-later
//! This computer's notifications, read from Windows so they can be shown on
//! the phone and on other computers (`Share this computer's notifications`).
//!
//! Windows only lets an app read other apps' toasts through
//! `UserNotificationListener`, which asks the user once ("Notification
//! access") and needs the app to run with package identity. Without either,
//! nothing is read and the reason is logged. Polling is used rather than the
//! change event, which needs a background task.

use std::{collections::HashSet, sync::OnceLock, time::Duration};

use windows::{
    UI::Notifications::{
        KnownNotificationBindings, NotificationKinds,
        Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
    },
    Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
};

use crate::{pc_notifications, settings};

const POLL: Duration = Duration::from_secs(2);

static STARTED: OnceLock<()> = OnceLock::new();

/// Starts reading notifications, once; it only forwards while the sharing
/// switch is on.
pub fn start() {
    STARTED.get_or_init(|| {
        let _ = std::thread::Builder::new().name("pc-notifications".into()).spawn(run);
    });
}

/// One toast as Windows holds it.
struct Toast {
    id: u32,
    app: String,
    title: String,
    text: String,
}

/// The first line of a toast is its title; the rest is its text.
fn split_lines(lines: Vec<String>) -> (String, String) {
    let mut lines = lines.into_iter().map(|l| l.trim().to_owned()).filter(|l| !l.is_empty());
    let title = lines.next().unwrap_or_default();
    (title, lines.collect::<Vec<_>>().join("\n"))
}

fn toasts(listener: &UserNotificationListener) -> windows::core::Result<Vec<Toast>> {
    let list = listener.GetNotificationsAsync(NotificationKinds::Toast)?.join()?;
    let mut out = Vec::new();
    for notification in list {
        let read = || -> windows::core::Result<Toast> {
            let id = notification.Id()?;
            let app = notification
                .AppInfo()
                .and_then(|info| info.DisplayInfo())
                .and_then(|display| display.DisplayName())
                .map(|name| name.to_string())
                .unwrap_or_default();
            let binding = notification.Notification()?.Visual()?.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
            let mut lines = Vec::new();
            for element in binding.GetTextElements()? {
                lines.push(element.Text()?.to_string());
            }
            let (title, text) = split_lines(lines);
            Ok(Toast { id, app, title, text })
        };
        // A toast in a shape we can't read is skipped, not fatal.
        if let Ok(toast) = read() {
            out.push(toast);
        }
    }
    Ok(out)
}

/// Whether the user lets this app read notifications, asking if not yet asked.
fn allowed(listener: &UserNotificationListener) -> bool {
    let mut status = listener.GetAccessStatus().unwrap_or(UserNotificationListenerAccessStatus::Unspecified);
    if status == UserNotificationListenerAccessStatus::Unspecified {
        status = listener
            .RequestAccessAsync()
            .and_then(|ask| ask.join())
            .unwrap_or(UserNotificationListenerAccessStatus::Unspecified);
    }
    status == UserNotificationListenerAccessStatus::Allowed
}

fn run() {
    // SAFETY: plain COM startup for this thread.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut seen: HashSet<u32> = HashSet::new();
    // The first look after the switch goes on only notes what is already in
    // the notification list, so old toasts are not all sent at once.
    let mut primed = false;
    let mut warned = false;
    loop {
        std::thread::sleep(POLL);
        if !settings::forward_pc_notifications_to_phone() {
            primed = false;
            continue;
        }
        let listener = match UserNotificationListener::Current() {
            Ok(listener) => listener,
            Err(e) => {
                if !std::mem::replace(&mut warned, true) {
                    tracing::warn!(error = %e, "Windows notification access is not available to this app");
                }
                continue;
            }
        };
        if !allowed(&listener) {
            if !std::mem::replace(&mut warned, true) {
                tracing::warn!(
                    "can't share this computer's notifications: Windows has not allowed notification access \
                     (Settings > Privacy & security > Notifications), or the app runs without package identity"
                );
            }
            continue;
        }
        warned = false;
        let current = match toasts(&listener) {
            Ok(current) => current,
            Err(e) => {
                tracing::debug!(error = %e, "couldn't read this computer's notifications");
                continue;
            }
        };
        let ids: HashSet<u32> = current.iter().map(|t| t.id).collect();
        if primed {
            for toast in current.iter().filter(|t| !seen.contains(&t.id)) {
                pc_notifications::forward_pc_toast(&toast.app, &toast.title, &toast.text);
            }
        }
        seen = ids;
        primed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_line_is_the_title_and_the_rest_the_text() {
        let (title, text) = split_lines(vec!["Ana".into(), " hello ".into(), "".into(), "see you".into()]);
        assert_eq!(title, "Ana");
        assert_eq!(text, "hello\nsee you");
        assert_eq!(split_lines(vec![]), (String::new(), String::new()));
    }
}
