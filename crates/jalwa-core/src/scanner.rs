//! Directory scanner — walks directories, probes audio files, extracts metadata.

use std::path::{Path, PathBuf};

use crate::{JalwaError, MediaItem, Result};
use tarang_core::MediaInfo;

/// Supported audio/media file extensions
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "mp4", "mkv", "webm", "aac", "opus",
];

/// A scanned file with extracted metadata
pub struct ScannedFile {
    pub path: PathBuf,
    pub info: MediaInfo,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

/// Scan a directory recursively for supported media files.
pub fn scan_directory(path: &Path) -> Result<Vec<ScannedFile>> {
    if !path.is_dir() {
        return Err(JalwaError::Scanner(format!(
            "not a directory: {}",
            path.display()
        )));
    }

    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        let ext = entry_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext {
            Some(e) if SUPPORTED_EXTENSIONS.contains(&e.as_str()) => {}
            _ => continue,
        }

        match scan_file(entry_path) {
            Ok(scanned) => results.push(scanned),
            Err(e) => {
                tracing::warn!(path = %entry_path.display(), error = %e, "skipping file");
            }
        }
    }

    Ok(results)
}

/// Scan a single file: probe with tarang, then extract rich tags with lofty.
fn scan_file(path: &Path) -> Result<ScannedFile> {
    // Probe with tarang for duration/codec/format info
    let file = std::fs::File::open(path)?;
    let info = tarang_audio::probe_audio(file).map_err(JalwaError::Tarang)?;

    // Extract rich tags with lofty
    let (title, artist, album) = match lofty::read_from_path(path) {
        Ok(tagged_file) => {
            use lofty::prelude::*;
            let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
            match tag {
                Some(t) => (
                    t.title().map(|s| s.to_string()),
                    t.artist().map(|s| s.to_string()),
                    t.album().map(|s| s.to_string()),
                ),
                None => (None, None, None),
            }
        }
        Err(_) => (None, None, None),
    };

    Ok(ScannedFile {
        path: path.to_path_buf(),
        info,
        title,
        artist,
        album,
    })
}

/// Convert a scanned file to a MediaItem, preferring lofty tags over probe-derived metadata.
pub fn scanned_to_media_item(scanned: ScannedFile) -> MediaItem {
    let mut item = MediaItem::from_probe(scanned.path, &scanned.info);

    // Override with richer lofty tags
    if let Some(title) = scanned.title {
        item.title = title;
    }
    if scanned.artist.is_some() {
        item.artist = scanned.artist;
    }
    if scanned.album.is_some() {
        item.album = scanned.album;
    }

    item
}
