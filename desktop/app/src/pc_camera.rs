// SPDX-License-Identifier: GPL-3.0-or-later
//! This computer's camera as another computer's webcam
//! (docs/protocol/pc-sharing.md §2).
//!
//! A paired computer asks with `webcam.start`; when camera sharing is on,
//! this computer opens its camera, encodes H.264 with the encoders screen
//! sharing uses, and streams it the way a phone streams its camera. The
//! other computer shows and decodes it as it does a phone's.
//!
//! Capture is Media Foundation on Windows. On Linux it is `ffmpeg` reading
//! the first working `/dev/video*`, the way the microphone uses `parec`.

#![cfg_attr(windows, allow(unsafe_code))]

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use noddy_core::{
    DeviceId, MirrorSend, PacketKind, WEBCAM_CAMERA_FRONT, WEBCAM_H264, WebcamConfig, WebcamStart,
};

use crate::{bridge::app::show_message, core_host, i18n::tr, pc_sharing::name_of};

/// The largest picture a shared camera sends: what a call needs, and what
/// the encoders keep up with.
const MAX_WIDTH: u32 = 1920;
const MAX_HEIGHT: u32 = 1080;
const MIN_WIDTH: u32 = 160;
const MIN_HEIGHT: u32 = 120;

/// How long a new session waits for the one it replaces to let go of the camera.
const HANDOVER: Duration = Duration::from_secs(2);

/// The camera's one session: who uses it and how to end it.
struct Session {
    peer: DeviceId,
    stop: Arc<AtomicBool>,
    /// The next frame must be a keyframe (the other computer lost its place).
    keyframe: Arc<AtomicBool>,
    /// Set when the session's thread has closed the camera and the stream.
    done: Arc<AtomicBool>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Whether this computer has a camera to share.
pub fn is_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/dev")
            .map(|e| e.flatten().any(|e| e.file_name().to_string_lossy().starts_with("video")))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

/// The picture size to ask for: inside what is shared, and even (H.264).
pub(crate) fn frame_size(width: u32, height: u32) -> (u32, u32) {
    let w = width.clamp(MIN_WIDTH, MAX_WIDTH) & !1;
    let h = height.clamp(MIN_HEIGHT, MAX_HEIGHT) & !1;
    (w, h)
}

/// A paired computer asked for this one's camera, or changed what it asked
/// for while using it (`webcam.start`).
pub fn requested(peer: &DeviceId, options: &WebcamStart) -> Result<(), String> {
    if !crate::settings::share_pc_camera() {
        return Err("camera sharing is off on this computer".into());
    }
    let (width, height) = frame_size(options.width, options.height);
    let fps = options.fps.clamp(1, 30);
    let bitrate = options.bitrate.clamp(500_000, 8_000_000);
    let session = Session {
        peer: *peer,
        stop: Arc::new(AtomicBool::new(false)),
        keyframe: Arc::new(AtomicBool::new(false)),
        done: Arc::new(AtomicBool::new(false)),
    };
    let (stop, keyframe, done) = (session.stop.clone(), session.keyframe.clone(), session.done.clone());
    let previous = SESSION.lock().unwrap_or_else(|e| e.into_inner()).replace(session);
    let announce = previous.as_ref().is_none_or(|p| p.peer != *peer);
    let wait_for = previous.map(|p| {
        p.stop.store(true, Ordering::SeqCst);
        p.done
    });
    let peer = *peer;
    std::thread::Builder::new()
        .name("noddy-pc-camera".into())
        .spawn(move || {
            // The camera can have one user: wait until the earlier one is done with it.
            if let Some(wait_for) = wait_for {
                let started = Instant::now();
                while !wait_for.load(Ordering::SeqCst) && started.elapsed() < HANDOVER {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            if !stop.load(Ordering::SeqCst) {
                run(peer, width, height, fps, bitrate, &stop, &keyframe);
            }
            done.store(true, Ordering::SeqCst);
            let mut session = SESSION.lock().unwrap_or_else(|e| e.into_inner());
            if session.as_ref().is_some_and(|s| Arc::ptr_eq(&s.stop, &stop)) {
                *session = None;
            }
        })
        .map_err(|e| e.to_string())?;
    if announce {
        show_message(tr!("PcSharing", "%1 is using this computer's camera.", name_of(&peer)));
    }
    Ok(())
}

/// The computer stopped using this one's camera (`webcam.stop`).
pub fn stop_requested(peer: &DeviceId) {
    if let Some(s) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|s| s.peer == *peer) {
        s.stop.store(true, Ordering::SeqCst);
    }
}

/// The other computer's decoder needs a fresh keyframe (`webcam.keyframe`).
pub fn keyframe_requested(peer: &DeviceId) {
    if let Some(s) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|s| s.peer == *peer) {
        s.keyframe.store(true, Ordering::SeqCst);
    }
}

/// Stops the camera for whoever is using it (sharing was turned off).
pub fn stop_all() {
    if let Some(s) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        s.stop.store(true, Ordering::SeqCst);
    }
}

fn run(
    peer: DeviceId,
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
    stop: &AtomicBool,
    keyframe: &AtomicBool,
) {
    let Some(node) = core_host::node() else { return };
    let fail = |what: String| {
        tracing::warn!(%peer, error = %what, "this computer's camera can't be shared");
        show_message(tr!("PcSharing", "This computer's camera can't be used: %1", what));
    };
    let mut camera = match Camera::open(width, height, fps) {
        Ok(camera) => camera,
        Err(e) => {
            // Tell the other computer: it is waiting for a picture.
            if let Ok(sender) = core_host::host().runtime.block_on(node.webcam_open(peer)) {
                sender.close();
            }
            fail(e);
            return;
        }
    };
    let (width, height) = camera.size();
    let mut encoder = match Encoder::new(width, height, fps, bitrate) {
        Ok(encoder) => encoder,
        Err(e) => {
            if let Ok(sender) = core_host::host().runtime.block_on(node.webcam_open(peer)) {
                sender.close();
            }
            fail(e);
            return;
        }
    };
    let Ok(sender) = core_host::host().runtime.block_on(node.webcam_open(peer)) else {
        tracing::warn!(%peer, "couldn't open the webcam stream");
        return;
    };
    let config = WebcamConfig {
        codec: WEBCAM_H264.into(),
        width,
        height,
        camera: WEBCAM_CAMERA_FRONT.into(),
        fps,
        max_height: height,
    };
    if matches!(sender.send(PacketKind::Config, 0, config.to_cbor()), MirrorSend::Closed) {
        return;
    }
    tracing::info!(%peer, width, height, fps, bitrate, "sharing this computer's camera");

    let started = Instant::now();
    let dur_hns = (10_000_000 / u64::from(fps)) as i64;
    // The first picture is a keyframe the decoder can start from.
    keyframe.store(true, Ordering::SeqCst);
    while !stop.load(Ordering::SeqCst) {
        let frame = match camera.next() {
            Ok(frame) => frame,
            Err(e) => {
                if !stop.load(Ordering::SeqCst) {
                    fail(e);
                }
                break;
            }
        };
        let hns = (started.elapsed().as_nanos() / 100) as i64;
        if keyframe.swap(false, Ordering::SeqCst) {
            // An encoder that can't ask for one still sends them from time to time.
            let _ = encoder.request_keyframe();
        }
        match encoder.encode(frame, hns, dur_hns, stop) {
            Ok(units) => {
                let mut closed = false;
                for (is_keyframe, data) in units {
                    let kind = if is_keyframe { PacketKind::Keyframe } else { PacketKind::Frame };
                    match sender.send(kind, (hns / 10) as u64, data) {
                        MirrorSend::Closed => {
                            closed = true;
                            break;
                        }
                        MirrorSend::NeedKeyframe => keyframe.store(true, Ordering::SeqCst),
                        _ => {}
                    }
                }
                if closed {
                    break;
                }
            }
            Err(e) => {
                if !stop.load(Ordering::SeqCst) {
                    fail(e);
                }
                break;
            }
        }
    }
    sender.close();
    tracing::info!(%peer, "stopped sharing this computer's camera");
}

// ---- H.264 ----

#[cfg(windows)]
struct Encoder(crate::pc_mirror::ScreenEncoder);

#[cfg(windows)]
impl Encoder {
    fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
        crate::pc_mirror::ScreenEncoder::new_for_codec("h264", width, height, fps, bitrate)
            .map(Self)
            .map_err(|e| e.to_string())
    }

    fn encode(
        &mut self,
        nv12: &[u8],
        hns: i64,
        dur_hns: i64,
        stop: &AtomicBool,
    ) -> Result<Vec<(bool, Vec<u8>)>, String> {
        self.0.encode(nv12, hns, dur_hns, stop).map_err(|e| e.to_string())
    }

    fn request_keyframe(&self) -> bool {
        self.0.request_keyframe()
    }
}

#[cfg(target_os = "linux")]
struct Encoder(crate::pc_mirror::OpenH264Encoder);

#[cfg(target_os = "linux")]
impl Encoder {
    fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
        crate::pc_mirror::OpenH264Encoder::new(width, height, fps, bitrate).map(Self)
    }

    fn encode(
        &mut self,
        nv12: &[u8],
        hns: i64,
        dur_hns: i64,
        stop: &AtomicBool,
    ) -> Result<Vec<(bool, Vec<u8>)>, String> {
        self.0.encode(nv12, hns, dur_hns, stop)
    }

    fn request_keyframe(&self) -> bool {
        self.0.request_keyframe()
    }
}

// ---- Capture: Windows ----

#[cfg(windows)]
struct Camera {
    reader: windows::Win32::Media::MediaFoundation::IMFSourceReader,
    source: windows::Win32::Media::MediaFoundation::IMFMediaSource,
    activate: windows::Win32::Media::MediaFoundation::IMFActivate,
    size: (u32, u32),
    frame: Vec<u8>,
}

#[cfg(windows)]
impl Camera {
    /// Opens the first camera that gives a picture, as NV12 near `width` x `height`.
    fn open(width: u32, height: u32, fps: u32) -> Result<Camera, String> {
        use windows::Win32::{
            Media::MediaFoundation::{
                IMFActivate, IMFAttributes, IMFMediaSource, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID, MFCreateAttributes, MFEnumDeviceSources,
            },
            System::Com::CoTaskMemFree,
        };

        crate::win::h264::ensure_mf_started().map_err(|e| e.to_string())?;
        // SAFETY: Media Foundation calls on live objects; the array of
        // activation objects it returns is read once, each taken by value, and
        // then freed.
        let devices: Vec<IMFActivate> = unsafe {
            let mut attributes: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attributes, 1).map_err(|e| e.to_string())?;
            let attributes = attributes.ok_or_else(|| "Media Foundation gave no attributes".to_owned())?;
            attributes
                .SetGUID(&MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID)
                .map_err(|e| e.to_string())?;
            let mut list: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count = 0u32;
            MFEnumDeviceSources(&attributes, &mut list, &mut count).map_err(|e| e.to_string())?;
            let devices = (0..count as usize).filter_map(|i| std::ptr::read(list.add(i))).collect();
            CoTaskMemFree(Some(list.cast()));
            devices
        };
        if devices.is_empty() {
            return Err("no camera found".into());
        }
        let mut last = "no camera could be opened".to_owned();
        for activate in devices {
            // SAFETY: activating and configuring a device the system listed.
            let opened = unsafe {
                activate
                    .ActivateObject::<IMFMediaSource>()
                    .and_then(|source| Self::from_source(activate.clone(), source, width, height, fps))
            };
            match opened {
                Ok(camera) => return Ok(camera),
                Err(e) => last = e.to_string(),
            }
        }
        Err(last)
    }

    /// # Safety
    /// Calls Media Foundation on `source`, which must be a live camera source.
    unsafe fn from_source(
        activate: windows::Win32::Media::MediaFoundation::IMFActivate,
        source: windows::Win32::Media::MediaFoundation::IMFMediaSource,
        width: u32,
        height: u32,
        fps: u32,
    ) -> windows::core::Result<Camera> {
        use windows::Win32::{
            Foundation::E_FAIL,
            Media::MediaFoundation::{
                IMFAttributes, IMFMediaType, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE,
                MF_MT_SUBTYPE, MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,
                MF_SOURCE_READER_FIRST_VIDEO_STREAM, MFCreateAttributes, MFCreateMediaType,
                MFCreateSourceReaderFromMediaSource, MFMediaType_Video, MFVideoFormat_NV12,
            },
        };

        // SAFETY: Media Foundation calls on objects created here.
        unsafe {
            let mut attributes: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attributes, 1)?;
            let attributes = attributes.ok_or_else(|| windows::core::Error::from(E_FAIL))?;
            // Lets the reader convert and resize to what is asked for.
            attributes.SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)?;
            let reader = MFCreateSourceReaderFromMediaSource(&source, &attributes)?;
            let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
            let request: IMFMediaType = MFCreateMediaType()?;
            request.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            request.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
            request.SetUINT64(&MF_MT_FRAME_SIZE, (u64::from(width) << 32) | u64::from(height))?;
            request.SetUINT64(&MF_MT_FRAME_RATE, (u64::from(fps) << 32) | 1)?;
            reader.SetCurrentMediaType(stream, None, &request)?;
            // What the camera really gives: the size the encoder must be made for.
            let packed = reader.GetCurrentMediaType(stream)?.GetUINT64(&MF_MT_FRAME_SIZE)?;
            let size = ((packed >> 32) as u32 & !1, packed as u32 & !1);
            Ok(Camera { reader, source, activate, size, frame: Vec::new() })
        }
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The next picture, `width * height * 3 / 2` bytes of NV12.
    fn next(&mut self) -> Result<&[u8], String> {
        use windows::Win32::Media::MediaFoundation::{
            IMFSample, MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READERF_ENDOFSTREAM,
        };

        let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        let need = self.size.0 as usize * self.size.1 as usize * 3 / 2;
        loop {
            let mut flags = 0u32;
            let mut sample: Option<IMFSample> = None;
            // SAFETY: ReadSample with valid out-pointers; the buffer is read
            // only within its current length, between Lock and Unlock.
            unsafe {
                self.reader
                    .ReadSample(stream, 0, None, Some(&mut flags), None, Some(&mut sample))
                    .map_err(|e| e.to_string())?;
                if flags & (MF_SOURCE_READERF_ENDOFSTREAM.0 as u32) != 0 {
                    return Err("the camera was disconnected".into());
                }
                // A gap in the stream: wait for the next picture.
                let Some(sample) = sample else { continue };
                let buffer = sample.ConvertToContiguousBuffer().map_err(|e| e.to_string())?;
                let mut data: *mut u8 = std::ptr::null_mut();
                let mut length = 0u32;
                buffer.Lock(&mut data, None, Some(&mut length)).map_err(|e| e.to_string())?;
                let fits = !data.is_null() && length as usize >= need;
                if fits {
                    self.frame.clear();
                    self.frame.extend_from_slice(std::slice::from_raw_parts(data, need));
                }
                let _ = buffer.Unlock();
                if !fits {
                    return Err("this camera's picture can't be shared".into());
                }
            }
            return Ok(&self.frame);
        }
    }
}

#[cfg(windows)]
impl Drop for Camera {
    fn drop(&mut self) {
        // SAFETY: lets the camera go, so its light goes off.
        unsafe {
            let _ = self.source.Shutdown();
            let _ = self.activate.ShutdownObject();
        }
    }
}

// ---- Capture: Linux ----

#[cfg(target_os = "linux")]
struct Camera {
    child: std::process::Child,
    out: std::process::ChildStdout,
    size: (u32, u32),
    frame: Vec<u8>,
}

/// `ffmpeg` arguments that read `device` and write NV12 pictures of exactly
/// `width` x `height` to standard output: scaled to fit, with bars if the
/// shape differs.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn ffmpeg_args(device: &str, width: u32, height: u32, fps: u32) -> Vec<String> {
    let filter = format!(
        "scale={width}:{height}:force_original_aspect_ratio=decrease,\
         pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,format=nv12"
    );
    [
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "v4l2",
        "-framerate",
        &fps.to_string(),
        "-i",
        device,
        "-vf",
        &filter,
        "-r",
        &fps.to_string(),
        "-pix_fmt",
        "nv12",
        "-f",
        "rawvideo",
        "-",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The numbers in `/dev/videoN` names, in order, as paths.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn video_nodes(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut numbered: Vec<(u32, String)> = names
        .into_iter()
        .filter_map(|name| {
            let n = name.strip_prefix("video")?.parse::<u32>().ok()?;
            Some((n, format!("/dev/{name}")))
        })
        .collect();
    numbered.sort();
    numbered.into_iter().map(|(_, path)| path).collect()
}

#[cfg(target_os = "linux")]
impl Camera {
    fn open(width: u32, height: u32, fps: u32) -> Result<Camera, String> {
        let names = std::fs::read_dir("/dev")
            .map(|entries| {
                entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let nodes = video_nodes(names);
        if nodes.is_empty() {
            return Err("no camera found".into());
        }
        let mut last = "no camera could be opened".to_owned();
        // A camera has several nodes and only some give a picture: the first that does.
        for node in nodes {
            match Self::open_node(&node, width, height, fps) {
                Ok(camera) => return Ok(camera),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    fn open_node(node: &str, width: u32, height: u32, fps: u32) -> Result<Camera, String> {
        use std::process::{Command, Stdio};

        let mut child = Command::new("ffmpeg")
            .args(ffmpeg_args(node, width, height, fps))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("ffmpeg isn't installed ({e})"))?;
        let Some(out) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("ffmpeg gave no picture".into());
        };
        let mut camera = Camera {
            child,
            out,
            size: (width, height),
            frame: vec![0; width as usize * height as usize * 3 / 2],
        };
        // The first picture shows the node works (dropping the camera ends ffmpeg).
        camera.next().map(|_| ()).map_err(|_| format!("{node} gives no picture"))?;
        Ok(camera)
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The next picture, `width * height * 3 / 2` bytes of NV12.
    fn next(&mut self) -> Result<&[u8], String> {
        use std::io::Read;

        self.out.read_exact(&mut self.frame).map_err(|e| format!("the camera stopped ({e})"))?;
        Ok(&self.frame)
    }
}

#[cfg(target_os = "linux")]
impl Drop for Camera {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_kept_inside_what_is_shared_and_even() {
        assert_eq!(frame_size(1280, 720), (1280, 720));
        assert_eq!(frame_size(3840, 2160), (1920, 1080));
        assert_eq!(frame_size(0, 0), (160, 120));
        assert_eq!(frame_size(641, 481), (640, 480));
    }

    #[test]
    fn camera_nodes_come_in_order_and_only_video_ones() {
        let names = ["video10", "video2", "video0", "videoX", "media0", "video"].map(str::to_owned);
        assert_eq!(video_nodes(names), ["/dev/video0", "/dev/video2", "/dev/video10"]);
    }

    #[test]
    fn ffmpeg_is_told_to_scale_to_the_picture_asked_for() {
        let args = ffmpeg_args("/dev/video0", 1280, 720, 30);
        let at = |flag: &str| args.iter().position(|a| a == flag).map(|i| args[i + 1].as_str());
        assert_eq!(at("-i"), Some("/dev/video0"));
        assert_eq!(at("-framerate"), Some("30"));
        assert_eq!(at("-pix_fmt"), Some("nv12"));
        assert!(at("-vf").is_some_and(|f| f.contains("scale=1280:720") && f.contains("pad=1280:720")));
        assert_eq!(args.last().map(String::as_str), Some("-"));
    }
}
