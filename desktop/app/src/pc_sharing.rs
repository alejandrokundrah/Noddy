// SPDX-License-Identifier: GPL-3.0-or-later
//! One computer using another's camera, microphone, sound and notifications
//! (docs/protocol/pc-sharing.md).
//!
//! These use the messages phones already use, with a computer as the source:
//!
//! - **Microphone**: this computer streams its default input to a computer
//!   that asked, while microphone sharing is on.
//! - **Sound**: [`crate::deck::handle_set_audio`] and
//!   [`crate::pc_audio_stream`] already send this computer's sound to the one
//!   that asked; here is the other end, playing what a computer sends.
//! - **Camera** is in [`crate::pc_camera`], **notifications** in
//!   [`crate::pc_notifications`].
//!
//! Every kind is off until the user turns it on (`settings::share_pc_*`), and
//! each paired computer has its own switch in the core.

#![cfg_attr(windows, allow(unsafe_code))]
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use noddy_core::{
    DeviceId, DeviceKind, LinkState, MIRROR_PCM, MirrorAudioConfig, MirrorSend, NodeEvent, PacketKind,
};

use crate::{bridge::app::show_message, core_host, i18n::tr, state::Changes, win::audio_out::Player};

/// Whether a paired device is another computer (not a phone).
pub fn peer_is_computer(peer: &DeviceId) -> bool {
    core_host::host().hub.read(|s| {
        s.devices
            .iter()
            .find(|d| d.id == *peer)
            .is_some_and(|d| matches!(d.info.kind, DeviceKind::Desktop | DeviceKind::Laptop))
    })
}

/// The name a paired device gives itself, for messages.
pub(crate) fn name_of(peer: &DeviceId) -> String {
    core_host::host()
        .hub
        .read(|s| s.devices.iter().find(|d| d.id == *peer).map(|d| d.info.name.clone()).unwrap_or_default())
}

// ---- Microphone (this computer is the source) ----

/// What a computer hears from this one's microphone, if anyone does.
struct MicSession {
    peer: DeviceId,
    stop: Arc<AtomicBool>,
}

static MIC: Mutex<Option<MicSession>> = Mutex::new(None);

/// 48 kHz mono 16-bit, 10 ms at a time.
const MIC_RATE: u32 = 48_000;
const MIC_CHUNK_BYTES: usize = 960;

/// A paired computer asked for this one's microphone (`mic.start`).
pub fn mic_requested(peer: &DeviceId) -> Result<(), String> {
    if !crate::settings::share_pc_mic() {
        return Err("microphone sharing is off on this computer".into());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let previous =
        MIC.lock().unwrap_or_else(|e| e.into_inner()).replace(MicSession { peer: *peer, stop: stop.clone() });
    if let Some(previous) = previous {
        previous.stop.store(true, Ordering::SeqCst);
    }
    let peer = *peer;
    let worker_stop = stop.clone();
    std::thread::Builder::new()
        .name("noddy-pc-mic".into())
        .spawn(move || {
            run_mic(peer, &worker_stop);
            // Only when no newer session took over.
            let mut mic = MIC.lock().unwrap_or_else(|e| e.into_inner());
            if mic.as_ref().is_some_and(|m| Arc::ptr_eq(&m.stop, &worker_stop)) {
                *mic = None;
            }
        })
        .map_err(|e| e.to_string())?;
    show_message(tr!("PcSharing", "%1 is using this computer's microphone.", name_of(&peer)));
    Ok(())
}

/// The computer stopped using this one's microphone (`mic.stop`).
pub fn mic_stop_requested(peer: &DeviceId) {
    if let Some(m) = MIC.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|m| m.peer == *peer) {
        m.stop.store(true, Ordering::SeqCst);
    }
}

/// Stops the microphone for whoever is using it (sharing was turned off).
pub fn stop_mic_for_all() {
    if let Some(m) = MIC.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        m.stop.store(true, Ordering::SeqCst);
    }
}

fn run_mic(peer: DeviceId, stop: &AtomicBool) {
    let Some(node) = core_host::node() else { return };
    let Ok(sender) = core_host::host().runtime.block_on(node.mic_open(peer)) else {
        tracing::warn!(%peer, "couldn't open the microphone stream");
        return;
    };
    let config = MirrorAudioConfig { codec: MIRROR_PCM.into(), rate: MIC_RATE, channels: 1 };
    if matches!(sender.send(PacketKind::Config, 0, config.to_cbor()), MirrorSend::Closed) {
        return;
    }
    tracing::info!(%peer, "sharing this computer's microphone");
    let started = Instant::now();
    let result = capture_mic(stop, |chunk| {
        let time_us = started.elapsed().as_micros() as u64;
        !matches!(sender.send(PacketKind::Frame, time_us, chunk), MirrorSend::Closed)
    });
    if let Err(e) = result {
        tracing::warn!(%peer, error = %e, "the microphone stopped");
        show_message(tr!("PcSharing", "This computer's microphone can't be used: %1", e));
    }
    sender.close();
    tracing::info!(%peer, "stopped sharing this computer's microphone");
}

/// Reads the default input as 48 kHz mono 16-bit PCM in 10 ms pieces and
/// hands each to `emit` until it returns `false` or `stop` is set.
#[cfg(target_os = "linux")]
fn capture_mic(stop: &AtomicBool, mut emit: impl FnMut(Vec<u8>) -> bool) -> Result<(), String> {
    use std::{
        io::Read,
        process::{Command, Stdio},
    };

    // The default source: no device named.
    let mut child = Command::new("parec")
        .args(["--raw", "--format=s16le", "--rate=48000", "--channels=1", "--latency-msec=10"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .or_else(|_| {
            Command::new("pw-record")
                .args(["--raw", "--format=s16", "--rate=48000", "--channels=1", "-"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
        })
        .map_err(|e| format!("neither parec nor pw-record could start: {e}"))?;
    let Some(mut out) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("the recorder gave no sound".into());
    };
    let mut chunk = vec![0u8; MIC_CHUNK_BYTES];
    while !stop.load(Ordering::SeqCst) {
        if out.read_exact(&mut chunk).is_err() || !emit(chunk.clone()) {
            break;
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[cfg(windows)]
fn capture_mic(stop: &AtomicBool, mut emit: impl FnMut(Vec<u8>) -> bool) -> Result<(), String> {
    use std::time::Duration;

    use windows::Win32::{
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
            MMDeviceEnumerator, WAVE_FORMAT_PCM, WAVEFORMATEX, eCapture, eConsole,
        },
        System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx},
    };

    let format = WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: 1,
        nSamplesPerSec: MIC_RATE,
        nAvgBytesPerSec: MIC_RATE * 2,
        nBlockAlign: 2,
        wBitsPerSample: 16,
        cbSize: 0,
    };
    // SAFETY: COM calls on live objects in this thread's apartment; the
    // capture buffer is read only within its frame count, between GetBuffer
    // and ReleaseBuffer.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let devices: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let device = devices.GetDefaultAudioEndpoint(eCapture, eConsole).map_err(|e| e.to_string())?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
        // Windows converts the rate and channels from the device's own.
        client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                1_000_000,
                0,
                &format,
                None,
            )
            .map_err(|e| e.to_string())?;
        let capture: IAudioCaptureClient = client.GetService().map_err(|e| e.to_string())?;
        client.Start().map_err(|e| e.to_string())?;

        let mut pending: Vec<u8> = Vec::with_capacity(MIC_CHUNK_BYTES * 4);
        'capture: while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
            loop {
                let Ok(frames) = capture.GetNextPacketSize() else { break 'capture };
                if frames == 0 {
                    break;
                }
                let mut data: *mut u8 = std::ptr::null_mut();
                let mut count: u32 = 0;
                let mut flags: u32 = 0;
                if capture.GetBuffer(&mut data, &mut count, &mut flags, None, None).is_err() {
                    break 'capture;
                }
                let bytes = count as usize * 2;
                if (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0 || data.is_null() {
                    pending.resize(pending.len() + bytes, 0);
                } else {
                    pending.extend_from_slice(std::slice::from_raw_parts(data, bytes));
                }
                let _ = capture.ReleaseBuffer(count);
                while pending.len() >= MIC_CHUNK_BYTES {
                    let rest = pending.split_off(MIC_CHUNK_BYTES);
                    let chunk = std::mem::replace(&mut pending, rest);
                    if !emit(chunk) {
                        break 'capture;
                    }
                }
            }
        }
        let _ = client.Stop();
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn capture_mic(_stop: &AtomicBool, _emit: impl FnMut(Vec<u8>) -> bool) -> Result<(), String> {
    Err("this system can't share its microphone yet".into())
}

// ---- Sound (this computer plays another's) ----

/// Whether another computer's sound is being asked for or is playing here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Asking,
    Playing,
}

struct Listening {
    peer: DeviceId,
    phase: Phase,
    /// Opened on the first frame, in the rate and channels it came in.
    player: Option<(u32, u8, Player)>,
}

static LISTENING: Mutex<Option<Listening>> = Mutex::new(None);

/// The computer whose sound plays here and how far it got: `(device, "asking" | "playing")`.
pub fn sound_state() -> Option<(DeviceId, &'static str)> {
    LISTENING.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|l| {
        (
            l.peer,
            match l.phase {
                Phase::Asking => "asking",
                Phase::Playing => "playing",
            },
        )
    })
}

fn sound_changed() {
    core_host::host().hub.changed(Changes::MEDIA);
}

/// Starts or stops playing another computer's sound here.
pub fn set_sound(peer: DeviceId, on: bool) {
    if !on {
        stop_sound(peer);
        return;
    }
    let Some(node) = core_host::node() else { return };
    let previous = {
        let mut listening = LISTENING.lock().unwrap_or_else(|e| e.into_inner());
        listening.replace(Listening { peer, phase: Phase::Asking, player: None })
    };
    // One computer's sound at a time: the earlier one is told to stop.
    if let Some(previous) = previous.filter(|p| p.peer != peer) {
        let node = node.clone();
        core_host::spawn(async move {
            let _ = node.set_pc_audio(previous.peer, Some(noddy_core::deck::PHONE_SPEAKER_OFF), None).await;
        });
    }
    sound_changed();
    core_host::spawn(async move {
        match node.set_pc_audio(peer, Some(noddy_core::deck::PHONE_SPEAKER_ON), None).await {
            Ok(()) => {}
            Err(e) => {
                clear_sound(peer);
                tracing::info!(%peer, error = %e, "couldn't play the computer's sound");
                show_message(match e {
                    noddy_core::Error::Denied => {
                        tr!(
                            "PcSharing",
                            "%1 isn't sharing its sound. Turn on Share sound there.",
                            name_of(&peer)
                        )
                    }
                    noddy_core::Error::Unsupported => {
                        tr!("PcSharing", "Update Noddy on %1 to play its sound here.", name_of(&peer))
                    }
                    _ => tr!("PcSharing", "%1's sound can't be played now.", name_of(&peer)),
                });
            }
        }
    });
}

fn stop_sound(peer: DeviceId) {
    let was = {
        let mut listening = LISTENING.lock().unwrap_or_else(|e| e.into_inner());
        if listening.as_ref().is_some_and(|l| l.peer == peer) { listening.take() } else { None }
    };
    if was.is_none() {
        return;
    }
    sound_changed();
    if let Some(node) = core_host::node() {
        core_host::spawn(async move {
            let _ = node.set_pc_audio(peer, Some(noddy_core::deck::PHONE_SPEAKER_OFF), None).await;
        });
    }
}

fn clear_sound(peer: DeviceId) {
    let cleared = {
        let mut listening = LISTENING.lock().unwrap_or_else(|e| e.into_inner());
        if listening.as_ref().is_some_and(|l| l.peer == peer) { listening.take().is_some() } else { false }
    };
    if cleared {
        sound_changed();
    }
}

/// Plays the sound of the computer that was asked, and nobody else's: frames
/// reach this computer from any paired one, so anything else is dropped.
pub fn on_event(event: &NodeEvent) {
    match event {
        NodeEvent::PcAudioFrame { device, sample_rate, channels, pcm } => {
            let mut listening = LISTENING.lock().unwrap_or_else(|e| e.into_inner());
            let Some(l) = listening.as_mut().filter(|l| l.peer == *device) else { return };
            let reopen = l.player.as_ref().is_none_or(|(rate, ch, _)| rate != sample_rate || ch != channels);
            if reopen {
                l.player = Player::start(*sample_rate, u16::from(*channels))
                    .ok()
                    .map(|p| (*sample_rate, *channels, p));
            }
            if let Some((_, _, player)) = l.player.as_ref() {
                player.push(pcm.clone());
            }
            if l.phase != Phase::Playing {
                l.phase = Phase::Playing;
                drop(listening);
                sound_changed();
            }
        }
        NodeEvent::LinkChanged { device, link } if !matches!(link, LinkState::Online { .. }) => {
            clear_sound(*device);
        }
        _ => {}
    }
}

/// Stops sending this computer's sound to computers (sound sharing was turned off).
/// A phone using it as its speaker is left alone.
pub fn stop_sound_to_computers() {
    if crate::pc_audio_stream::target().is_some_and(|t| peer_is_computer(&t)) {
        crate::pc_audio_stream::set_target(None);
        crate::pc_audio_stream::set_enabled(false);
    }
}

// ---- Camera ----

/// Stops the camera for whoever is using it (sharing was turned off).
pub fn stop_camera_for_all() {
    crate::pc_camera::stop_all();
}
