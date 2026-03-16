//! SQLite persistence for the Jalwa media library.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::{JalwaError, Library, MediaItem, MediaType, Playlist, Result};
use tarang_core::{AudioCodec, ContainerFormat, VideoCodec};

/// Low-level database handle.
pub struct LibraryDb {
    conn: Connection,
}

impl LibraryDb {
    /// Open (or create) the library database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn =
            Connection::open(path).map_err(|e| JalwaError::Database(format!("open: {e}")))?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Run migrations to create/update schema.
    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS media_items (
                id TEXT PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                duration_ms INTEGER,
                format TEXT NOT NULL,
                audio_codec TEXT,
                video_codec TEXT,
                media_type TEXT NOT NULL,
                added_at TEXT NOT NULL,
                last_played TEXT,
                play_count INTEGER NOT NULL DEFAULT 0,
                rating INTEGER,
                tags TEXT
            );
            CREATE TABLE IF NOT EXISTS playlists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                is_smart INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS playlist_items (
                playlist_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                FOREIGN KEY (playlist_id) REFERENCES playlists(id),
                FOREIGN KEY (item_id) REFERENCES media_items(id)
            );
            CREATE TABLE IF NOT EXISTS scan_paths (
                path TEXT PRIMARY KEY
            );
            ",
            )
            .map_err(|e| JalwaError::Database(format!("migrate: {e}")))?;
        Ok(())
    }

    /// Load the entire library from the database into memory.
    pub fn load_library(&self) -> Result<Library> {
        let mut library = Library::new();

        // Load items
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, path, title, artist, album, duration_ms, format,
                    audio_codec, video_codec, media_type, added_at, last_played,
                    play_count, rating, tags
             FROM media_items",
            )
            .map_err(|e| JalwaError::Database(format!("prepare: {e}")))?;

        let items = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let path: String = row.get(1)?;
                let title: String = row.get(2)?;
                let artist: Option<String> = row.get(3)?;
                let album: Option<String> = row.get(4)?;
                let duration_ms: Option<i64> = row.get(5)?;
                let format: String = row.get(6)?;
                let audio_codec: Option<String> = row.get(7)?;
                let video_codec: Option<String> = row.get(8)?;
                let media_type: String = row.get(9)?;
                let added_at: String = row.get(10)?;
                let last_played: Option<String> = row.get(11)?;
                let play_count: u32 = row.get(12)?;
                let rating: Option<u8> = row.get(13)?;
                let tags_json: Option<String> = row.get(14)?;

                Ok((
                    id,
                    path,
                    title,
                    artist,
                    album,
                    duration_ms,
                    format,
                    audio_codec,
                    video_codec,
                    media_type,
                    added_at,
                    last_played,
                    play_count,
                    rating,
                    tags_json,
                ))
            })
            .map_err(|e| JalwaError::Database(format!("query: {e}")))?;

        for row in items {
            let (
                id,
                path,
                title,
                artist,
                album,
                duration_ms,
                format,
                audio_codec,
                video_codec,
                media_type,
                added_at,
                last_played,
                play_count,
                rating,
                tags_json,
            ) = row.map_err(|e| JalwaError::Database(format!("row: {e}")))?;

            let item = MediaItem {
                id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4()),
                path: PathBuf::from(path),
                title,
                artist,
                album,
                duration: duration_ms.map(|ms| Duration::from_millis(ms as u64)),
                format: parse_format(&format),
                audio_codec: audio_codec.as_deref().map(parse_audio_codec),
                video_codec: video_codec.as_deref().map(parse_video_codec),
                media_type: if media_type == "Video" {
                    MediaType::Video
                } else {
                    MediaType::Audio
                },
                added_at: DateTime::parse_from_rfc3339(&added_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_played: last_played.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                play_count,
                rating,
                tags: tags_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                art_mime: None, // Art is extracted on scan, not stored in DB
                art_data: None,
            };
            library.add_item(item);
        }

        // Load playlists
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, description, is_smart, created_at, modified_at FROM playlists",
            )
            .map_err(|e| JalwaError::Database(format!("prepare playlists: {e}")))?;

        let playlists = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let description: Option<String> = row.get(2)?;
                let is_smart: bool = row.get(3)?;
                let created_at: String = row.get(4)?;
                let modified_at: String = row.get(5)?;
                Ok((id, name, description, is_smart, created_at, modified_at))
            })
            .map_err(|e| JalwaError::Database(format!("query playlists: {e}")))?;

        for row in playlists {
            let (id, name, description, is_smart, created_at, modified_at) =
                row.map_err(|e| JalwaError::Database(format!("row: {e}")))?;

            let mut playlist = Playlist::new(&name);
            playlist.id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4());
            playlist.description = description;
            playlist.is_smart = is_smart;
            playlist.created_at = DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            playlist.modified_at = DateTime::parse_from_rfc3339(&modified_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            // Load playlist items
            let mut item_stmt = self
                .conn
                .prepare(
                    "SELECT item_id FROM playlist_items WHERE playlist_id = ? ORDER BY position",
                )
                .map_err(|e| JalwaError::Database(format!("prepare playlist items: {e}")))?;

            let item_ids = item_stmt
                .query_map(params![id], |row| {
                    let item_id: String = row.get(0)?;
                    Ok(item_id)
                })
                .map_err(|e| JalwaError::Database(format!("query playlist items: {e}")))?;

            for item_id in item_ids {
                if let Ok(id_str) = item_id
                    && let Ok(uuid) = Uuid::parse_str(&id_str) {
                        playlist.items.push(uuid);
                    }
            }

            library.playlists.push(playlist);
        }

        // Load scan paths
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM scan_paths")
            .map_err(|e| JalwaError::Database(format!("prepare scan_paths: {e}")))?;

        let paths = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                Ok(PathBuf::from(path))
            })
            .map_err(|e| JalwaError::Database(format!("query scan_paths: {e}")))?;

        for p in paths.flatten() {
            library.scan_paths.push(p);
        }

        Ok(library)
    }

    /// Save a media item (upsert by path).
    pub fn save_item(&self, item: &MediaItem) -> Result<()> {
        let tags_json = serde_json::to_string(&item.tags).unwrap_or_else(|_| "[]".to_string());
        self.conn
            .execute(
                "INSERT OR REPLACE INTO media_items
             (id, path, title, artist, album, duration_ms, format, audio_codec, video_codec,
              media_type, added_at, last_played, play_count, rating, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    item.id.to_string(),
                    item.path.to_string_lossy().to_string(),
                    item.title,
                    item.artist,
                    item.album,
                    item.duration.map(|d| d.as_millis() as i64),
                    format!("{:?}", item.format),
                    item.audio_codec.map(|c| format!("{:?}", c)),
                    item.video_codec.map(|c| format!("{:?}", c)),
                    format!("{:?}", item.media_type),
                    item.added_at.to_rfc3339(),
                    item.last_played.map(|lp| lp.to_rfc3339()),
                    item.play_count,
                    item.rating,
                    tags_json,
                ],
            )
            .map_err(|e| JalwaError::Database(format!("save item: {e}")))?;
        Ok(())
    }

    /// Delete a media item by UUID.
    pub fn delete_item(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM media_items WHERE id = ?",
                params![id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("delete item: {e}")))?;
        // Also remove from playlist_items
        self.conn
            .execute(
                "DELETE FROM playlist_items WHERE item_id = ?",
                params![id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("delete playlist item refs: {e}")))?;
        Ok(())
    }

    /// Update play count and last_played for an item.
    pub fn update_play_count(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute(
                "UPDATE media_items SET play_count = play_count + 1, last_played = ? WHERE id = ?",
                params![Utc::now().to_rfc3339(), id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("update play count: {e}")))?;
        Ok(())
    }

    /// Update rating for an item.
    pub fn update_rating(&self, id: Uuid, rating: Option<u8>) -> Result<()> {
        self.conn
            .execute(
                "UPDATE media_items SET rating = ? WHERE id = ?",
                params![rating, id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("update rating: {e}")))?;
        Ok(())
    }

    /// Save a playlist (upsert).
    pub fn save_playlist(&self, playlist: &Playlist) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO playlists (id, name, description, is_smart, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                playlist.id.to_string(),
                playlist.name,
                playlist.description,
                playlist.is_smart,
                playlist.created_at.to_rfc3339(),
                playlist.modified_at.to_rfc3339(),
            ],
        ).map_err(|e| JalwaError::Database(format!("save playlist: {e}")))?;

        // Replace playlist items
        self.conn
            .execute(
                "DELETE FROM playlist_items WHERE playlist_id = ?",
                params![playlist.id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("clear playlist items: {e}")))?;

        for (pos, item_id) in playlist.items.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO playlist_items (playlist_id, item_id, position) VALUES (?1, ?2, ?3)",
                params![playlist.id.to_string(), item_id.to_string(), pos as i64],
            ).map_err(|e| JalwaError::Database(format!("save playlist item: {e}")))?;
        }

        Ok(())
    }

    /// Delete a playlist by UUID.
    pub fn delete_playlist(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM playlist_items WHERE playlist_id = ?",
                params![id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("delete playlist items: {e}")))?;
        self.conn
            .execute(
                "DELETE FROM playlists WHERE id = ?",
                params![id.to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("delete playlist: {e}")))?;
        Ok(())
    }

    /// Save a scan path.
    pub fn save_scan_path(&self, path: &Path) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO scan_paths (path) VALUES (?)",
                params![path.to_string_lossy().to_string()],
            )
            .map_err(|e| JalwaError::Database(format!("save scan path: {e}")))?;
        Ok(())
    }

    /// Load all scan paths.
    pub fn load_scan_paths(&self) -> Result<Vec<PathBuf>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM scan_paths")
            .map_err(|e| JalwaError::Database(format!("prepare: {e}")))?;
        let paths = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                Ok(PathBuf::from(path))
            })
            .map_err(|e| JalwaError::Database(format!("query: {e}")))?;

        let mut result = Vec::new();
        for path in paths.flatten() {
            result.push(path);
        }
        Ok(result)
    }
}

/// Persistent library — wraps Library + LibraryDb for write-through persistence.
pub struct PersistentLibrary {
    pub library: Library,
    db: LibraryDb,
}

impl PersistentLibrary {
    /// Open a persistent library from a database file.
    pub fn open(db_path: &Path) -> Result<Self> {
        let db = LibraryDb::open(db_path)?;
        let library = db.load_library()?;
        Ok(Self { library, db })
    }

    /// Add a media item to both memory and database.
    pub fn add_item(&mut self, item: MediaItem) -> Result<Uuid> {
        self.db.save_item(&item)?;
        let id = self.library.add_item(item);
        Ok(id)
    }

    /// Remove a media item.
    pub fn remove_item(&mut self, id: Uuid) -> Result<bool> {
        self.db.delete_item(id)?;
        Ok(self.library.remove(id))
    }

    /// Update play count.
    pub fn update_play_count(&mut self, id: Uuid) -> Result<()> {
        self.db.update_play_count(id)?;
        if let Some(item) = self.library.find_by_id_mut(id) {
            item.play_count += 1;
            item.last_played = Some(Utc::now());
        }
        Ok(())
    }

    /// Update rating.
    pub fn update_rating(&mut self, id: Uuid, rating: Option<u8>) -> Result<()> {
        self.db.update_rating(id, rating)?;
        if let Some(item) = self.library.find_by_id_mut(id) {
            item.rating = rating;
        }
        Ok(())
    }

    /// Save a playlist.
    pub fn save_playlist(&mut self, playlist: &Playlist) -> Result<()> {
        self.db.save_playlist(playlist)?;
        // Update in-memory
        if let Some(pos) = self
            .library
            .playlists
            .iter()
            .position(|p| p.id == playlist.id)
        {
            self.library.playlists[pos] = playlist.clone();
        } else {
            self.library.playlists.push(playlist.clone());
        }
        Ok(())
    }

    /// Delete a playlist.
    pub fn delete_playlist(&mut self, id: Uuid) -> Result<()> {
        self.db.delete_playlist(id)?;
        self.library.playlists.retain(|p| p.id != id);
        Ok(())
    }

    /// Add a scan path.
    pub fn add_scan_path(&mut self, path: PathBuf) -> Result<()> {
        self.db.save_scan_path(&path)?;
        self.library.add_scan_path(path);
        Ok(())
    }

    /// Get a reference to the underlying db for direct operations.
    pub fn db(&self) -> &LibraryDb {
        &self.db
    }
}

// ---- Format parsing helpers ----

fn parse_format(s: &str) -> ContainerFormat {
    match s {
        "Mp4" => ContainerFormat::Mp4,
        "Mkv" => ContainerFormat::Mkv,
        "WebM" => ContainerFormat::WebM,
        "Ogg" => ContainerFormat::Ogg,
        "Wav" => ContainerFormat::Wav,
        "Flac" => ContainerFormat::Flac,
        "Mp3" => ContainerFormat::Mp3,
        "Avi" => ContainerFormat::Avi,
        _ => ContainerFormat::Mp3, // fallback
    }
}

fn parse_audio_codec(s: &str) -> AudioCodec {
    match s {
        "Pcm" => AudioCodec::Pcm,
        "Mp3" => AudioCodec::Mp3,
        "Aac" => AudioCodec::Aac,
        "Flac" => AudioCodec::Flac,
        "Vorbis" => AudioCodec::Vorbis,
        "Opus" => AudioCodec::Opus,
        "Alac" => AudioCodec::Alac,
        "Wma" => AudioCodec::Wma,
        _ => AudioCodec::Pcm,
    }
}

fn parse_video_codec(s: &str) -> VideoCodec {
    match s {
        "H264" => VideoCodec::H264,
        "H265" => VideoCodec::H265,
        "Vp8" => VideoCodec::Vp8,
        "Vp9" => VideoCodec::Vp9,
        "Av1" => VideoCodec::Av1,
        "Theora" => VideoCodec::Theora,
        _ => VideoCodec::H264,
    }
}

/// Get the default database path (~/.local/share/jalwa/library.db).
pub fn default_db_path() -> PathBuf {
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        });
    data_dir.join("jalwa").join("library.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tarang_core::*;

    fn make_test_item(title: &str, artist: &str) -> MediaItem {
        MediaItem {
            id: Uuid::new_v4(),
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: Some("Album".to_string()),
            duration: Some(Duration::from_secs(200)),
            format: ContainerFormat::Flac,
            audio_codec: Some(AudioCodec::Flac),
            video_codec: None,
            media_type: MediaType::Audio,
            added_at: Utc::now(),
            last_played: None,
            play_count: 0,
            rating: None,
            tags: vec!["rock".to_string()],
            art_mime: None,
            art_data: None,
        }
    }

    #[test]
    fn db_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let db = LibraryDb::open(&tmp).unwrap();

        let item = make_test_item("Song", "Artist");
        let id = item.id;
        db.save_item(&item).unwrap();

        let lib = db.load_library().unwrap();
        assert_eq!(lib.items.len(), 1);
        assert_eq!(lib.items[0].title, "Song");
        assert_eq!(lib.items[0].artist, Some("Artist".to_string()));
        assert_eq!(lib.items[0].tags, vec!["rock".to_string()]);

        db.delete_item(id).unwrap();
        let lib = db.load_library().unwrap();
        assert!(lib.items.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn db_playlist_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let db = LibraryDb::open(&tmp).unwrap();

        let item = make_test_item("Song", "Artist");
        db.save_item(&item).unwrap();

        let mut playlist = Playlist::new("Test");
        playlist.add(item.id);
        db.save_playlist(&playlist).unwrap();

        let lib = db.load_library().unwrap();
        assert_eq!(lib.playlists.len(), 1);
        assert_eq!(lib.playlists[0].name, "Test");
        assert_eq!(lib.playlists[0].items.len(), 1);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn persistent_library() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        {
            let mut plib = PersistentLibrary::open(&tmp).unwrap();
            let item = make_test_item("Song", "Artist");
            plib.add_item(item).unwrap();
            assert_eq!(plib.library.items.len(), 1);
        }
        // Reopen — data should persist
        {
            let plib = PersistentLibrary::open(&tmp).unwrap();
            assert_eq!(plib.library.items.len(), 1);
            assert_eq!(plib.library.items[0].title, "Song");
        }
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn db_update_play_count() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let db = LibraryDb::open(&tmp).unwrap();
        let item = make_test_item("Song", "Artist");
        let id = item.id;
        db.save_item(&item).unwrap();

        db.update_play_count(id).unwrap();
        db.update_play_count(id).unwrap();

        let lib = db.load_library().unwrap();
        assert_eq!(lib.items[0].play_count, 2);
        assert!(lib.items[0].last_played.is_some());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn db_update_rating() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let db = LibraryDb::open(&tmp).unwrap();
        let item = make_test_item("Song", "Artist");
        let id = item.id;
        db.save_item(&item).unwrap();

        db.update_rating(id, Some(4)).unwrap();
        let lib = db.load_library().unwrap();
        assert_eq!(lib.items[0].rating, Some(4));

        db.update_rating(id, None).unwrap();
        let lib = db.load_library().unwrap();
        assert_eq!(lib.items[0].rating, None);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn db_scan_paths() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let db = LibraryDb::open(&tmp).unwrap();

        db.save_scan_path(Path::new("/home/user/Music")).unwrap();
        db.save_scan_path(Path::new("/home/user/Music")).unwrap(); // duplicate
        db.save_scan_path(Path::new("/home/user/Videos")).unwrap();

        let paths = db.load_scan_paths().unwrap();
        assert_eq!(paths.len(), 2);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn db_delete_playlist() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let db = LibraryDb::open(&tmp).unwrap();

        let playlist = Playlist::new("Temp");
        let pl_id = playlist.id;
        db.save_playlist(&playlist).unwrap();

        db.delete_playlist(pl_id).unwrap();
        let lib = db.load_library().unwrap();
        assert!(lib.playlists.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn persistent_library_play_count_and_rating() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let mut plib = PersistentLibrary::open(&tmp).unwrap();
        let item = make_test_item("Song", "Artist");
        let id = plib.add_item(item).unwrap();

        plib.update_play_count(id).unwrap();
        assert_eq!(plib.library.find_by_id(id).unwrap().play_count, 1);

        plib.update_rating(id, Some(5)).unwrap();
        assert_eq!(plib.library.find_by_id(id).unwrap().rating, Some(5));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn persistent_library_playlist_crud() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let mut plib = PersistentLibrary::open(&tmp).unwrap();

        // Add an item so we can reference it in the playlist
        let item = make_test_item("Song", "Artist");
        let item_id = plib.add_item(item).unwrap();

        let playlist = Playlist::new("Favs");
        let pl_id = playlist.id;
        plib.save_playlist(&playlist).unwrap();
        assert_eq!(plib.library.playlists.len(), 1);

        // Update existing — add the real item
        let mut updated = playlist.clone();
        updated.add(item_id);
        plib.save_playlist(&updated).unwrap();
        assert_eq!(plib.library.playlists.len(), 1);
        assert_eq!(plib.library.playlists[0].items.len(), 1);

        plib.delete_playlist(pl_id).unwrap();
        assert!(plib.library.playlists.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn persistent_library_scan_path_and_remove() {
        let tmp = std::env::temp_dir().join(format!("jalwa_test_{}.db", Uuid::new_v4()));
        let mut plib = PersistentLibrary::open(&tmp).unwrap();

        plib.add_scan_path(PathBuf::from("/music")).unwrap();
        assert_eq!(plib.library.scan_paths.len(), 1);

        let item = make_test_item("Song", "Artist");
        let id = plib.add_item(item).unwrap();
        assert!(plib.remove_item(id).unwrap());
        assert!(plib.library.items.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_helpers() {
        assert_eq!(parse_format("Mp4"), ContainerFormat::Mp4);
        assert_eq!(parse_format("Mkv"), ContainerFormat::Mkv);
        assert_eq!(parse_format("WebM"), ContainerFormat::WebM);
        assert_eq!(parse_format("Ogg"), ContainerFormat::Ogg);
        assert_eq!(parse_format("Wav"), ContainerFormat::Wav);
        assert_eq!(parse_format("Flac"), ContainerFormat::Flac);
        assert_eq!(parse_format("Mp3"), ContainerFormat::Mp3);
        assert_eq!(parse_format("Avi"), ContainerFormat::Avi);
        assert_eq!(parse_format("unknown"), ContainerFormat::Mp3);

        assert_eq!(parse_audio_codec("Pcm"), AudioCodec::Pcm);
        assert_eq!(parse_audio_codec("Mp3"), AudioCodec::Mp3);
        assert_eq!(parse_audio_codec("Aac"), AudioCodec::Aac);
        assert_eq!(parse_audio_codec("Flac"), AudioCodec::Flac);
        assert_eq!(parse_audio_codec("Vorbis"), AudioCodec::Vorbis);
        assert_eq!(parse_audio_codec("Opus"), AudioCodec::Opus);
        assert_eq!(parse_audio_codec("Alac"), AudioCodec::Alac);
        assert_eq!(parse_audio_codec("Wma"), AudioCodec::Wma);
        assert_eq!(parse_audio_codec("???"), AudioCodec::Pcm);

        assert_eq!(parse_video_codec("H264"), VideoCodec::H264);
        assert_eq!(parse_video_codec("H265"), VideoCodec::H265);
        assert_eq!(parse_video_codec("Vp8"), VideoCodec::Vp8);
        assert_eq!(parse_video_codec("Vp9"), VideoCodec::Vp9);
        assert_eq!(parse_video_codec("Av1"), VideoCodec::Av1);
        assert_eq!(parse_video_codec("Theora"), VideoCodec::Theora);
        assert_eq!(parse_video_codec("???"), VideoCodec::H264);
    }

    #[test]
    fn default_db_path_ends_correctly() {
        let path = default_db_path();
        assert!(path.ends_with("jalwa/library.db"));
    }
}
