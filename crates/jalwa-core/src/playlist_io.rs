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
    use crate::test_fixtures::make_media_item;
    use uuid::Uuid;

    #[test]
    fn m3u_roundtrip() {
        let mut lib = Library::new();
        let item1 = make_media_item("Song A", "Artist 1", 200);
        let item2 = make_media_item("Song B", "Artist 2", 180);
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
        assert_eq!(paths[0], PathBuf::from("/music/Song A.flac"));
        assert_eq!(paths[1], PathBuf::from("/music/Song B.flac"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn save_and_load_m3u_roundtrip() {
        let mut lib = Library::new();
        let item1 = make_media_item("Alpha", "Band X", 120);
        let item2 = make_media_item("Beta", "Band Y", 240);
        let item3 = make_media_item("Gamma", "Band Z", 300);
        let id1 = item1.id;
        let id2 = item2.id;
        let id3 = item3.id;
        lib.add_item(item1);
        lib.add_item(item2);
        lib.add_item(item3);

        let mut playlist = Playlist::new("Roundtrip");
        playlist.add(id1);
        playlist.add(id2);
        playlist.add(id3);

        let tmp = std::env::temp_dir().join(format!("jalwa_rt_{}.m3u", Uuid::new_v4()));
        save_m3u(&playlist, &lib, &tmp).unwrap();

        let paths = load_m3u(&tmp).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], PathBuf::from("/music/Alpha.flac"));
        assert_eq!(paths[1], PathBuf::from("/music/Beta.flac"));
        assert_eq!(paths[2], PathBuf::from("/music/Gamma.flac"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn load_m3u_empty_file() {
        let tmp = std::env::temp_dir().join(format!("jalwa_empty_{}.m3u", Uuid::new_v4()));
        std::fs::write(&tmp, "").unwrap();

        let paths = load_m3u(&tmp).unwrap();
        assert!(paths.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn load_m3u_with_comments() {
        let tmp = std::env::temp_dir().join(format!("jalwa_comments_{}.m3u", Uuid::new_v4()));
        let content = "#EXTM3U\n\
                        #EXTINF:120,Artist - Title\n\
                        /music/track1.flac\n\
                        # This is a comment\n\
                        /music/track2.flac\n\
                        \n\
                        /music/track3.flac\n";
        std::fs::write(&tmp, content).unwrap();

        let paths = load_m3u(&tmp).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], PathBuf::from("/music/track1.flac"));
        assert_eq!(paths[1], PathBuf::from("/music/track2.flac"));
        assert_eq!(paths[2], PathBuf::from("/music/track3.flac"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn load_m3u_nonexistent_file() {
        let bad_path = PathBuf::from(format!("/tmp/jalwa_no_such_file_{}.m3u", Uuid::new_v4()));
        let result = load_m3u(&bad_path);
        assert!(result.is_err());
    }

    #[test]
    fn save_m3u_creates_file() {
        let lib = Library::new();
        let playlist = Playlist::new("Empty");

        let tmp = std::env::temp_dir().join(format!("jalwa_create_{}.m3u", Uuid::new_v4()));
        assert!(!tmp.exists());

        save_m3u(&playlist, &lib, &tmp).unwrap();
        assert!(tmp.exists());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn save_m3u_empty_playlist() {
        let lib = Library::new();
        let playlist = Playlist::new("Empty");

        let tmp = std::env::temp_dir().join(format!("jalwa_emptypl_{}.m3u", Uuid::new_v4()));
        save_m3u(&playlist, &lib, &tmp).unwrap();

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content.trim(), "#EXTM3U");

        let _ = std::fs::remove_file(&tmp);
    }
}
