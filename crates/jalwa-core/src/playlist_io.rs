//! M3U playlist import/export.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use crate::{JalwaError, Library, Playlist, Result};

/// Save a playlist as an M3U file.
pub fn save_m3u(playlist: &Playlist, library: &Library, path: &Path) -> Result<()> {
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "#EXTM3U").map_err(JalwaError::Io)?;

    for item_id in &playlist.items {
        if let Some(item) = library.find_by_id(*item_id) {
            let duration_secs = item.duration.map(|d| d.as_secs() as i64).unwrap_or(-1);
            let title = if let Some(ref artist) = item.artist {
                format!("{} - {}", artist, item.title)
            } else {
                item.title.clone()
            };
            writeln!(file, "#EXTINF:{},{}", duration_secs, title).map_err(JalwaError::Io)?;
            writeln!(file, "{}", item.path.display()).map_err(JalwaError::Io)?;
        }
    }

    Ok(())
}

/// Load an M3U file, returning the list of file paths.
pub fn load_m3u(path: &Path) -> Result<Vec<PathBuf>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut paths = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        paths.push(PathBuf::from(trimmed));
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MediaItem, MediaType};
    use std::time::Duration;
    use tarang::core::{AudioCodec, ContainerFormat};
    use uuid::Uuid;

    fn make_item(title: &str, artist: &str, path: &str) -> MediaItem {
        MediaItem {
            id: Uuid::new_v4(),
            path: PathBuf::from(path),
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: None,
            duration: Some(Duration::from_secs(200)),
            format: ContainerFormat::Flac,
            audio_codec: Some(AudioCodec::Flac),
            video_codec: None,
            media_type: MediaType::Audio,
            added_at: chrono::Utc::now(),
            last_played: None,
            play_count: 0,
            rating: None,
            tags: Vec::new(),
            art_mime: None,
            art_data: None,
        }
    }

    #[test]
    fn m3u_roundtrip() {
        let mut lib = Library::new();
        let item1 = make_item("Song A", "Artist 1", "/music/a.flac");
        let item2 = make_item("Song B", "Artist 2", "/music/b.flac");
        let id1 = item1.id;
        let id2 = item2.id;
        lib.add_item(item1);
        lib.add_item(item2);

        let mut playlist = Playlist::new("Test");
        playlist.add(id1);
        playlist.add(id2);

        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.m3u", Uuid::new_v4()));
        save_m3u(&playlist, &lib, &tmp).unwrap();

        let paths = load_m3u(&tmp).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/music/a.flac"));
        assert_eq!(paths[1], PathBuf::from("/music/b.flac"));

        let _ = std::fs::remove_file(&tmp);
    }
}
