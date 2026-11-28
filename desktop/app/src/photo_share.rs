// SPDX-License-Identifier: GPL-3.0-or-later
//! This computer's photos and screenshots for another paired computer
//! (`photos.read`, docs/protocol/photos.md): the same gallery messages a
//! phone answers, with this computer's Pictures folder as the source.
//!
//! Albums are the Pictures folder itself and the folders in it (Screenshots,
//! Camera Roll, …). Only still images are listed. Nothing is offered until the
//! user turns on "Share this computer's photos" (`settings::share_pc_photos`),
//! and each paired computer still has its own Photos switch in the core.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, UNIX_EPOCH},
};

use noddy_core::{FileSource, OutgoingFile, PhotoAlbum, PhotoItem, PhotoThumb};

use crate::photos::fingerprint;

/// Still images the gallery lists.
const IMAGE_EXTENSIONS: &[&str] =
    &["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tif", "tiff", "avif", "jxl"];
/// How deep under Pictures folders are looked through.
const MAX_DEPTH: usize = 4;
/// Most photos listed, newest kept.
const MAX_ITEMS: usize = 20_000;
/// How long a listing is reused before the folders are looked through again.
const FRESH_FOR: Duration = Duration::from_secs(30);
/// Images larger than this aren't decoded for a thumbnail.
const MAX_THUMB_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
/// Thumbnails kept in memory between requests.
const MAX_CACHED_THUMBS: usize = 1_500;

struct Entry {
    item: PhotoItem,
    path: PathBuf,
    modified: i64,
}

struct Index {
    root: PathBuf,
    built: Instant,
    /// Newest first, then by ID (highest first).
    entries: Vec<Entry>,
    by_id: HashMap<String, usize>,
    albums: Vec<PhotoAlbum>,
}

static INDEX: Mutex<Option<Index>> = Mutex::new(None);
/// Thumbnails by photo ID, with the file's modified time they were made from.
static THUMBS: Mutex<Option<HashMap<String, (i64, Vec<u8>)>>> = Mutex::new(None);

/// Forgets the listing and thumbnails (sharing was turned off).
pub fn forget() {
    *INDEX.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *THUMBS.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Whether this computer can offer its photos at all (the setting aside).
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
pub fn is_available() -> bool {
    cfg!(any(windows, target_os = "linux")) && pictures_root().is_some()
}

fn pictures_root() -> Option<PathBuf> {
    dirs::picture_dir().filter(|p| p.is_dir())
}

fn with_index<T>(f: impl FnOnce(&Index) -> T) -> Result<T, String> {
    if !crate::settings::share_pc_photos() {
        return Err("photo sharing is off on this computer".into());
    }
    let root = pictures_root().ok_or("this computer has no Pictures folder")?;
    let mut guard = INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let stale = guard.as_ref().is_none_or(|i| i.root != root || i.built.elapsed() > FRESH_FOR);
    if stale {
        *guard = Some(build(root));
    }
    Ok(f(guard.as_ref().ok_or("no listing")?))
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

fn hidden(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('$') || name.eq_ignore_ascii_case("desktop.ini")
}

/// A folder's path under Pictures, with `/` between names ("" for Pictures).
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn album_id(rel_dir: &str) -> String {
    format!("pca{:016x}", fingerprint(rel_dir))
}

fn photo_id(rel_path: &str) -> String {
    format!("pc:{:016x}", fingerprint(rel_path))
}

fn millis(time: std::io::Result<std::time::SystemTime>) -> i64 {
    time.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

fn build(root: PathBuf) -> Index {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    // (folder, how deep it is under Pictures)
    let mut stack = vec![(root.clone(), 0usize)];
    let mut album_names: HashMap<String, String> = HashMap::new();
    while let Some((dir, depth)) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        let rel_dir = relative(&root, &dir);
        let album = album_id(&rel_dir);
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if hidden(&name) {
                continue;
            }
            // Links aren't followed: they could lead out of Pictures.
            let Ok(kind) = entry.file_type() else { continue };
            let path = entry.path();
            if kind.is_dir() {
                if depth < MAX_DEPTH {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            if !kind.is_file() || !is_image(&path) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let rel = relative(&root, &path);
            let id = photo_id(&rel);
            if !seen.insert(id.clone()) {
                continue;
            }
            let modified = millis(meta.modified());
            let date = match millis(meta.created()) {
                // A copy keeps when it was taken as its modified time, and is
                // "created" when copied: the earlier of the two is the photo's.
                created if created > 0 && (modified == 0 || created < modified) => created,
                _ => modified,
            };
            album_names.entry(album.clone()).or_insert_with(|| {
                dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Pictures".into())
            });
            entries.push(Entry {
                item: PhotoItem {
                    id,
                    name: noddy_core::safe_file_name(&name),
                    date,
                    size: meta.len(),
                    width: 0,
                    height: 0,
                    duration: None,
                    album: Some(album.clone()),
                },
                path,
                modified,
            });
        }
    }
    entries.sort_by(|a, b| (b.item.date, &b.item.id).cmp(&(a.item.date, &a.item.id)));
    entries.truncate(MAX_ITEMS);
    let by_id = entries.iter().enumerate().map(|(i, e)| (e.item.id.clone(), i)).collect();
    let albums = albums_of(&entries, &album_names);
    Index { root, built: Instant::now(), entries, by_id, albums }
}

/// Screenshots and camera folders first, then the biggest.
fn albums_of(entries: &[Entry], names: &HashMap<String, String>) -> Vec<PhotoAlbum> {
    let mut albums: Vec<PhotoAlbum> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for entry in entries {
        let Some(id) = entry.item.album.as_deref() else { continue };
        match index.get(id) {
            Some(&i) => albums[i].count += 1,
            None => {
                index.insert(id, albums.len());
                albums.push(PhotoAlbum {
                    id: id.to_owned(),
                    name: names.get(id).cloned().unwrap_or_else(|| "Pictures".into()),
                    count: 1,
                    // Entries are newest first, so the first one seen is the cover.
                    cover: Some(entry.item.id.clone()),
                });
            }
        }
    }
    let rank = |name: &str| {
        let name = name.to_ascii_lowercase();
        if name.contains("screenshot") {
            0
        } else if name.contains("camera") {
            1
        } else {
            2
        }
    };
    albums.sort_by(|a, b| {
        rank(&a.name).cmp(&rank(&b.name)).then(b.count.cmp(&a.count)).then(a.name.cmp(&b.name))
    });
    albums
}

/// The albums (`photos.albums`).
pub fn albums() -> Result<Vec<PhotoAlbum>, String> {
    with_index(|index| index.albums.clone())
}

/// Photos in `album` (or all), newest first, after `before` (`photos.list`).
pub fn list(album: Option<&str>, before: Option<(i64, &str)>, limit: u32) -> Result<Vec<PhotoItem>, String> {
    with_index(|index| {
        index
            .entries
            .iter()
            .filter(|e| album.is_none_or(|a| e.item.album.as_deref() == Some(a)))
            .filter(|e| before.is_none_or(|(date, id)| (e.item.date, e.item.id.as_str()) < (date, id)))
            .take(limit as usize)
            .map(|e| e.item.clone())
            .collect()
    })
}

/// Small JPEG thumbnails; ones that can't be made are left out (`photos.thumbs`).
pub fn thumbs(ids: &[String]) -> Result<Vec<PhotoThumb>, String> {
    let wanted: Vec<(String, PathBuf, i64)> = with_index(|index| {
        ids.iter()
            .filter_map(|id| {
                let entry = &index.entries[*index.by_id.get(id)?];
                Some((id.clone(), entry.path.clone(), entry.modified))
            })
            .collect()
    })?;
    let mut out = Vec::with_capacity(wanted.len());
    for (id, path, modified) in wanted {
        let cached = THUMBS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|c| c.get(&id))
            .filter(|(made_from, _)| *made_from == modified)
            .map(|(_, data)| data.clone());
        let data = match cached {
            Some(data) => data,
            None => match thumbnail(&path) {
                Ok(data) => {
                    let mut cache = THUMBS.lock().unwrap_or_else(|e| e.into_inner());
                    let cache = cache.get_or_insert_with(HashMap::new);
                    if cache.len() >= MAX_CACHED_THUMBS {
                        cache.clear();
                    }
                    cache.insert(id.clone(), (modified, data.clone()));
                    data
                }
                Err(reason) => {
                    tracing::debug!(reason, "no thumbnail for a shared photo");
                    continue;
                }
            },
        };
        out.push(PhotoThumb { id, data });
    }
    Ok(out)
}

/// A photo to send to the computer that asked (`photos.get`).
pub fn open(id: &str) -> Result<OutgoingFile, String> {
    let path = with_index(|index| index.by_id.get(id).map(|&i| index.entries[i].path.clone()))?
        .ok_or("no such photo")?;
    if !path.is_file() {
        return Err("it's gone".into());
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "photo".into());
    Ok(OutgoingFile { name: noddy_core::safe_file_name(&name), folder: None, source: FileSource::Path(path) })
}

fn thumbnail(path: &Path) -> Result<Vec<u8>, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_THUMB_SOURCE_BYTES {
        return Err("too large to preview".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    make_thumbnail(&bytes)
}

/// A JPEG of at most `PHOTO_MAX_THUMB_BYTES`: at 320 pixels on the longest
/// side, or smaller if that comes out too big.
#[cfg(any(windows, target_os = "linux"))]
fn make_thumbnail(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use crate::win::image;
    let bitmap = image::decode(bytes).map_err(|e| e.to_string())?;
    for side in [320, 200, 128] {
        let scaled = image::scale_to(&bitmap, side);
        #[cfg(windows)]
        let jpeg = image::encode_jpeg(&scaled).map_err(|e| e.to_string())?;
        #[cfg(target_os = "linux")]
        let jpeg = image::encode_jpeg(&scaled, 80).map_err(|e| e.to_string())?;
        if jpeg.len() <= noddy_core::PHOTO_MAX_THUMB_BYTES {
            return Ok(jpeg);
        }
    }
    Err("thumbnail too large".into())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn make_thumbnail(_bytes: &[u8]) -> Result<Vec<u8>, String> {
    Err("photo sharing isn't supported on this computer yet".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, album: &str, date: i64) -> Entry {
        Entry {
            item: PhotoItem {
                id: id.into(),
                name: format!("{id}.jpg"),
                date,
                size: 1,
                width: 0,
                height: 0,
                duration: None,
                album: Some(album.into()),
            },
            path: PathBuf::from(id),
            modified: date,
        }
    }

    #[test]
    fn ids_are_valid_photo_ids_and_stable() {
        // What the protocol takes as a photo ID: 1 to 64 printable ASCII bytes.
        let valid = |id: &str| (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_graphic());
        let id = photo_id("Screenshots/Screenshot 2026-11-28.png");
        assert_eq!(id, photo_id("Screenshots/Screenshot 2026-11-28.png"));
        assert!(valid(&id), "{id}");
        assert!(valid(&album_id("")));
        assert_ne!(album_id(""), album_id("Screenshots"));
    }

    #[test]
    fn only_still_images_are_listed() {
        assert!(is_image(Path::new("a/IMG_1.JPG")));
        assert!(is_image(Path::new("shot.png")));
        assert!(!is_image(Path::new("clip.mp4")));
        assert!(!is_image(Path::new("notes")));
        assert!(hidden(".thumbnails") && hidden("desktop.ini") && !hidden("Screenshots"));
    }

    #[test]
    fn albums_put_screenshots_first_with_the_newest_as_cover() {
        let entries = vec![entry("p3", "a1", 30), entry("p2", "a2", 20), entry("p1", "a1", 10)];
        let names = HashMap::from([
            ("a1".to_owned(), "Trips".to_owned()),
            ("a2".to_owned(), "Screenshots".to_owned()),
        ]);
        let albums = albums_of(&entries, &names);
        assert_eq!(albums.len(), 2);
        assert_eq!((albums[0].name.as_str(), albums[0].count), ("Screenshots", 1));
        assert_eq!(
            (albums[1].name.as_str(), albums[1].count, albums[1].cover.as_deref()),
            ("Trips", 2, Some("p3"))
        );
    }

    #[test]
    fn a_real_folder_is_indexed_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let shots = dir.path().join("Screenshots");
        std::fs::create_dir(&shots).unwrap();
        std::fs::write(dir.path().join("a.jpg"), b"x").unwrap();
        std::fs::write(shots.join("b.png"), b"x").unwrap();
        std::fs::write(shots.join("c.mp4"), b"x").unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::write(dir.path().join(".hidden").join("d.jpg"), b"x").unwrap();
        let index = build(dir.path().to_owned());
        let mut names: Vec<_> = index.entries.iter().map(|e| e.item.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["a.jpg", "b.png"]);
        assert_eq!(index.albums.len(), 2);
        assert!(index.albums.iter().any(|a| a.name == "Screenshots"));
    }
}
