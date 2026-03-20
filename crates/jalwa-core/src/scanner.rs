//! Directory scanner — walks directories, probes audio files, extracts metadata.

use std::path::{Path, PathBuf};

use crate::{JalwaError, MediaItem, Result};
use tarang::core::MediaInfo;

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
    /// Album art MIME type (e.g. "image/jpeg")
    pub art_mime: Option<String>,
    /// Raw album art bytes
    pub art_data: Option<Vec<u8>>,
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

/// Scan a single file: probe with tarang, then extract rich tags + album art with lofty.
fn scan_file(path: &Path) -> Result<ScannedFile> {
    // Probe with tarang for duration/codec/format info
    let file = std::fs::File::open(path)?;
    let info = tarang::audio::probe_audio(file).map_err(JalwaError::Tarang)?;

    // Extract rich tags + album art with lofty
    let (title, artist, album, art_mime, art_data) = match lofty::read_from_path(path) {
        Ok(tagged_file) => {
            use lofty::prelude::*;
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());
            match tag {
                Some(t) => {
                    let title = t.title().map(|s| s.to_string());
                    let artist = t.artist().map(|s| s.to_string());
                    let album = t.album().map(|s| s.to_string());

                    // Extract album art (prefer front cover)
                    let (art_mime, art_data) = extract_art(t.pictures());

                    (title, artist, album, art_mime, art_data)
                }
                None => (None, None, None, None, None),
            }
        }
        Err(_) => (None, None, None, None, None),
    };

    Ok(ScannedFile {
        path: path.to_path_buf(),
        info,
        title,
        artist,
        album,
        art_mime,
        art_data,
    })
}

/// Maximum embedded album art size (5 MB).
const MAX_ART_SIZE: usize = 5 * 1024 * 1024;

/// Extract the best album art picture from a list of tag pictures.
fn extract_art(pictures: &[lofty::picture::Picture]) -> (Option<String>, Option<Vec<u8>>) {
    if pictures.is_empty() {
        return (None, None);
    }

    // Prefer CoverFront, fall back to first picture
    let pic = pictures
        .iter()
        .find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront)
        .or_else(|| pictures.first());

    match pic {
        Some(p) => {
            let mime = p.mime_type().map(|m| m.as_str().to_string());
            let data = p.data().to_vec();
            if data.is_empty() || data.len() > MAX_ART_SIZE {
                (None, None)
            } else {
                (mime, Some(data))
            }
        }
        None => (None, None),
    }
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
    item.art_mime = scanned.art_mime;
    item.art_data = scanned.art_data;

    item
}

/// Check if a file extension is a supported media format.
pub fn is_supported_extension(ext: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_extensions() {
        assert!(is_supported_extension("mp3"));
        assert!(is_supported_extension("FLAC"));
        assert!(is_supported_extension("wav"));
        assert!(is_supported_extension("ogg"));
        assert!(is_supported_extension("m4a"));
        assert!(is_supported_extension("opus"));
        assert!(!is_supported_extension("txt"));
        assert!(!is_supported_extension("pdf"));
        assert!(!is_supported_extension("rs"));
    }

    #[test]
    fn scan_not_a_directory() {
        let result = scan_directory(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn scan_empty_directory() {
        let tmp = std::env::temp_dir().join(format!("jalwa_scan_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let result = scan_directory(&tmp).unwrap();
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_skips_non_media() {
        let tmp = std::env::temp_dir().join(format!("jalwa_scan_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("readme.txt"), "hello").unwrap();
        std::fs::write(tmp.join("code.rs"), "fn main() {}").unwrap();
        let result = scan_directory(&tmp).unwrap();
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn extract_art_empty() {
        let (mime, data) = extract_art(&[]);
        assert!(mime.is_none());
        assert!(data.is_none());
    }

    #[test]
    fn scanned_to_media_item_applies_tags() {
        use tarang::core::*;
        let info = MediaInfo {
            id: uuid::Uuid::new_v4(),
            format: ContainerFormat::Mp3,
            streams: vec![StreamInfo::Audio(AudioStreamInfo {
                codec: AudioCodec::Mp3,
                sample_rate: 44100,
                channels: 2,
                sample_format: SampleFormat::F32,
                bitrate: Some(320_000),
                duration: Some(std::time::Duration::from_secs(200)),
            })],
            duration: Some(std::time::Duration::from_secs(200)),
            file_size: Some(8_000_000),
            title: None,
            artist: None,
            album: None,
        };

        let scanned = ScannedFile {
            path: PathBuf::from("/music/track.mp3"),
            info,
            title: Some("My Song".to_string()),
            artist: Some("My Artist".to_string()),
            album: Some("My Album".to_string()),
            art_mime: Some("image/jpeg".to_string()),
            art_data: Some(vec![0xFF, 0xD8, 0xFF]),
        };

        let item = scanned_to_media_item(scanned);
        assert_eq!(item.title, "My Song");
        assert_eq!(item.artist, Some("My Artist".to_string()));
        assert_eq!(item.album, Some("My Album".to_string()));
        assert_eq!(item.art_mime, Some("image/jpeg".to_string()));
        assert!(item.art_data.is_some());
    }

    #[test]
    fn scanned_to_media_item_no_tags() {
        use tarang::core::*;
        let info = MediaInfo {
            id: uuid::Uuid::new_v4(),
            format: ContainerFormat::Flac,
            streams: vec![StreamInfo::Audio(AudioStreamInfo {
                codec: AudioCodec::Flac,
                sample_rate: 44100,
                channels: 2,
                sample_format: SampleFormat::I16,
                bitrate: None,
                duration: Some(std::time::Duration::from_secs(180)),
            })],
            duration: Some(std::time::Duration::from_secs(180)),
            file_size: Some(20_000_000),
            title: Some("Probe Title".to_string()),
            artist: Some("Probe Artist".to_string()),
            album: None,
        };

        let scanned = ScannedFile {
            path: PathBuf::from("/music/song.flac"),
            info,
            title: None,
            artist: None,
            album: None,
            art_mime: None,
            art_data: None,
        };

        let item = scanned_to_media_item(scanned);
        // Title falls back to filename stem since lofty title is None
        assert_eq!(item.title, "song");
        assert!(item.art_data.is_none());
    }

    /// Create a minimal valid WAV file for testing.
    fn make_wav(num_samples: u32, sample_rate: u32) -> Vec<u8> {
        let channels: u16 = 1;
        let bits: u16 = 16;
        let data_size = num_samples * channels as u32 * (bits as u32 / 8);
        let file_size = 36 + data_size;
        let byte_rate = sample_rate * channels as u32 * (bits as u32 / 8);
        let block_align = channels * (bits / 8);

        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for i in 0..num_samples {
            let t = i as f64 / sample_rate as f64;
            let s = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
            let s16 = (s * 16000.0) as i16;
            buf.extend_from_slice(&s16.to_le_bytes());
        }
        buf
    }

    #[test]
    fn scan_directory_with_wav() {
        let tmp = std::env::temp_dir().join(format!("jalwa_scan_wav_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let wav = make_wav(4410, 44100);
        std::fs::write(tmp.join("test.wav"), &wav).unwrap();
        // Also write a non-media file that should be skipped
        std::fs::write(tmp.join("notes.txt"), "skip me").unwrap();

        let results = scan_directory(&tmp).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("test.wav"));
        assert!(results[0].info.duration.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_file_and_convert() {
        let tmp = std::env::temp_dir().join(format!("jalwa_scan_convert_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let wav = make_wav(44100, 44100); // 1 second
        let wav_path = tmp.join("song.wav");
        std::fs::write(&wav_path, &wav).unwrap();

        let results = scan_directory(&tmp).unwrap();
        assert_eq!(results.len(), 1);

        let item = scanned_to_media_item(results.into_iter().next().unwrap());
        assert_eq!(item.title, "song"); // filename stem
        assert!(item.duration.is_some());
        let dur = item.duration.unwrap();
        assert!(dur.as_secs_f64() > 0.9 && dur.as_secs_f64() < 1.1);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
