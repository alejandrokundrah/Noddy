// SPDX-License-Identifier: GPL-3.0-or-later
//! Virtual extended monitor management on Windows: locates or attaches a
//! secondary / Indirect Display (IddCx) monitor at a custom resolution
//! (up to 3840x2160) positioned as an extended display, and restores it
//! when the session ends.

use windows::{
    Win32::{
        Foundation::POINTL,
        Graphics::Gdi::{
            CDS_NORESET, CDS_RESET, CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW, DEVMODEW,
            DISP_CHANGE_SUCCESSFUL, DISPLAY_DEVICE_ACTIVE, DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICEW,
            DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS,
            EnumDisplayDevicesW, EnumDisplaySettingsExW,
        },
    },
    core::PCWSTR,
};

use super::desktop_input::{ScreenRect, primary_monitor};

/// A virtual or secondary extended monitor attached for an extended display session.
pub struct VirtualMonitor {
    device_name: Vec<u16>,
    rect: ScreenRect,
    attached_by_us: bool,
}

impl VirtualMonitor {
    /// Returns the monitor's rectangle in virtual desktop physical pixels.
    pub fn rect(&self) -> ScreenRect {
        self.rect
    }

    /// Resizes this virtual monitor to `width` x `height` (clamped to `320..=3840` x `240..=2160`).
    pub fn resize(&mut self, width: u32, height: u32) -> Result<ScreenRect, String> {
        let w = width.clamp(320, 3840);
        let h = height.clamp(240, 2160);
        let rect = apply_mode(&self.device_name, w, h, self.rect.left, self.rect.top)?;
        self.rect = rect;
        Ok(rect)
    }
}

impl Drop for VirtualMonitor {
    fn drop(&mut self) {
        if self.attached_by_us {
            // Detach the virtual display from the desktop when the session ends.
            let mut dm = DEVMODEW {
                dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                dmFields: DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION,
                dmPelsWidth: 0,
                dmPelsHeight: 0,
                ..Default::default()
            };
            // SAFETY: `device_name` is NUL-terminated UTF-16 and `dm` is properly sized.
            unsafe {
                let _ = ChangeDisplaySettingsExW(
                    PCWSTR(self.device_name.as_ptr()),
                    Some(&raw mut dm),
                    None,
                    CDS_UPDATEREGISTRY | CDS_NORESET,
                    None,
                );
                let _ = ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_RESET, None);
            }
        }
    }
}

/// Returns `true` if a secondary or Indirect Display (IddCx) adapter is present
/// on this Windows PC that can be used as an extended monitor.
#[allow(dead_code)]
pub fn is_available() -> bool {
    find_extended_device().is_some()
}

/// Attaches (or reuses) a secondary/virtual extended monitor at `width` x `height`
/// positioned immediately to the right of the primary monitor.
pub fn acquire(width: u32, height: u32) -> Result<VirtualMonitor, String> {
    let w = width.clamp(320, 3840);
    let h = height.clamp(240, 2160);
    let (device_name, was_active) = find_extended_device().ok_or_else(|| {
        "No virtual or secondary display adapter is available on this PC. Install a Windows Indirect Display Driver (IddCx) or connect a secondary display.".to_owned()
    })?;
    let primary = primary_monitor();
    let origin_x = primary.left + primary.width;
    let origin_y = primary.top;
    let rect = apply_mode(&device_name, w, h, origin_x, origin_y)?;
    Ok(VirtualMonitor { device_name, rect, attached_by_us: !was_active })
}

fn find_extended_device() -> Option<(Vec<u16>, bool)> {
    let mut fallback_active: Option<(Vec<u16>, bool)> = None;
    let mut fallback_inactive: Option<(Vec<u16>, bool)> = None;

    for i in 0..32u32 {
        let mut dev =
            DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
        // SAFETY: `dev` is initialized with `cb` set to its byte size.
        let ok = unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut dev, 0).as_bool() };
        if !ok {
            break;
        }
        let is_primary = (dev.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE).0 != 0;
        if is_primary {
            continue;
        }
        let is_active = (dev.StateFlags & DISPLAY_DEVICE_ACTIVE).0 != 0;
        let name_len = dev.DeviceName.iter().position(|&c| c == 0).unwrap_or(dev.DeviceName.len());
        let mut name = dev.DeviceName[..name_len].to_vec();
        name.push(0);

        let desc_len = dev.DeviceString.iter().position(|&c| c == 0).unwrap_or(dev.DeviceString.len());
        let desc = String::from_utf16_lossy(&dev.DeviceString[..desc_len]).to_ascii_lowercase();
        let is_virtual = desc.contains("idd")
            || desc.contains("virtual")
            || desc.contains("parsec")
            || desc.contains("indirect")
            || desc.contains("noddy");

        if is_virtual {
            return Some((name, is_active));
        }
        if is_active && fallback_active.is_none() {
            fallback_active = Some((name, true));
        } else if !is_active && fallback_inactive.is_none() {
            fallback_inactive = Some((name, false));
        }
    }

    fallback_active.or(fallback_inactive)
}

fn apply_mode(device_name: &[u16], width: u32, height: u32, x: i32, y: i32) -> Result<ScreenRect, String> {
    let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
    // SAFETY: `device_name` is NUL-terminated UTF-16 and `dm` is properly sized.
    let has_current = unsafe {
        EnumDisplaySettingsExW(
            PCWSTR(device_name.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut dm,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
        .as_bool()
    };
    if !has_current {
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    }
    dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
    dm.dmPelsWidth = width;
    dm.dmPelsHeight = height;
    dm.Anonymous1.Anonymous2.dmPosition = POINTL { x, y };

    // SAFETY: `device_name` is NUL-terminated UTF-16 and `dm` is properly initialized.
    let res = unsafe {
        ChangeDisplaySettingsExW(
            PCWSTR(device_name.as_ptr()),
            Some(&raw mut dm),
            None,
            CDS_UPDATEREGISTRY | CDS_NORESET,
            None,
        )
    };
    if res != DISP_CHANGE_SUCCESSFUL {
        return Err(format!("display mode {width}x{height} rejected (code {})", res.0));
    }
    // SAFETY: commits pending display layout changes.
    let commit = unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_RESET, None) };
    if commit != DISP_CHANGE_SUCCESSFUL {
        return Err(format!("could not apply extended display layout (code {})", commit.0));
    }
    Ok(ScreenRect { left: x, top: y, width: width as i32, height: height as i32 })
}
