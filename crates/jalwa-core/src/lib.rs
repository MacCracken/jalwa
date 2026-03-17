//! jalwa-core — Core types for the Jalwa media player
//!
//! Media library, playlists, play queue, playback state, and settings.

pub mod db;
pub mod playlist_io;
pub mod scanner;
pub mod watcher;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tarang_core::{AudioCodec, ContainerFormat, MediaInfo, VideoCodec};
use uuid::Uuid;

/// A media item in the library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: Uuid,
    pub path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub format: ContainerFormat,
    pub audio_codec: Option<AudioCodec>,
    pub video_codec: Option<VideoCodec>,
    pub media_type: MediaType,
    pub added_at: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub play_count: u32,
    pub rating: Option<u8>,
    pub tags: Vec<String>,
    /// Album art MIME type (e.g. "image/jpeg") if embedded art was found.
    pub art_mime: Option<String>,
    /// Album art data (raw bytes, typically JPEG or PNG).
    #[serde(skip)]
    pub art_data: Option<Vec<u8>>,
}

impl MediaItem {
    /// Create a new media item from a file path and tarang probe info
    pub fn from_probe(path: PathBuf, info: &MediaInfo) -> Self {
        let audio_codec = info.audio_streams().first().map(|a| a.codec);
        let video_codec = info.video_streams().first().map(|v| v.codec);
        let media_type = if info.has_video() {
            MediaType::Video
        } else {
            MediaType::Audio
        };

        Self {
            id: Uuid::new_v4(),
            title: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            artist: info.artist.clone(),
            album: info.album.clone(),
            duration: info.duration,
            format: info.format,
            audio_codec,
            video_codec,
            media_type,
            path,
            added_at: Utc::now(),
            last_played: None,
            play_count: 0,
            rating: None,
            tags: Vec::new(),
            art_mime: None,
            art_data: None,
        }
    }

    pub fn is_audio(&self) -> bool {
        self.media_type == MediaType::Audio
    }

    pub fn is_video(&self) -> bool {
        self.media_type == MediaType::Video
    }
}

/// Type of media content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Audio,
    Video,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Audio => write!(f, "audio"),
            Self::Video => write!(f, "video"),
        }
    }
}

/// A playlist — ordered collection of media items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Uuid,
    pub name: String,
    pub items: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub description: Option<String>,
    pub is_smart: bool,
}

impl Playlist {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            items: Vec::new(),
            created_at: now,
            modified_at: now,
            description: None,
            is_smart: false,
        }
    }

    pub fn add(&mut self, item_id: Uuid) {
        self.items.push(item_id);
        self.modified_at = Utc::now();
    }

    pub fn remove(&mut self, item_id: Uuid) -> bool {
        let before = self.items.len();
        self.items.retain(|id| *id != item_id);
        if self.items.len() != before {
            self.modified_at = Utc::now();
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.modified_at = Utc::now();
    }
}

/// Play queue with current position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayQueue {
    pub items: Vec<Uuid>,
    pub position: Option<usize>,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,
}

impl PlayQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            position: None,
            repeat_mode: RepeatMode::Off,
            shuffle: false,
        }
    }

    pub fn enqueue(&mut self, item_id: Uuid) {
        self.items.push(item_id);
        if self.position.is_none() && !self.items.is_empty() {
            self.position = Some(0);
        }
    }

    pub fn enqueue_many(&mut self, ids: impl IntoIterator<Item = Uuid>) {
        let was_empty = self.items.is_empty();
        self.items.extend(ids);
        if self.position.is_none() && was_empty && !self.items.is_empty() {
            self.position = Some(0);
        }
    }

    pub fn current(&self) -> Option<Uuid> {
        self.position.and_then(|p| self.items.get(p).copied())
    }

    pub fn advance(&mut self) -> Option<Uuid> {
        let pos = self.position?;
        let next = pos + 1;
        if next < self.items.len() {
            self.position = Some(next);
            Some(self.items[next])
        } else if self.repeat_mode == RepeatMode::All && !self.items.is_empty() {
            self.position = Some(0);
            Some(self.items[0])
        } else {
            None
        }
    }

    pub fn go_back(&mut self) -> Option<Uuid> {
        let pos = self.position?;
        if pos > 0 {
            self.position = Some(pos - 1);
            Some(self.items[pos - 1])
        } else if self.repeat_mode == RepeatMode::All && !self.items.is_empty() {
            let last = self.items.len() - 1;
            self.position = Some(last);
            Some(self.items[last])
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.position = None;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for PlayQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Repeat mode for the play queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

impl RepeatMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::One,
            Self::One => Self::All,
            Self::All => Self::Off,
        }
    }
}

impl std::fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::One => write!(f, "one"),
            Self::All => write!(f, "all"),
        }
    }
}

/// Current playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

impl std::fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Playing => write!(f, "playing"),
            Self::Paused => write!(f, "paused"),
            Self::Buffering => write!(f, "buffering"),
        }
    }
}

/// Playback status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
    pub current_item: Option<Uuid>,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: f32,
    pub muted: bool,
}

impl PlaybackStatus {
    pub fn stopped() -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_item: None,
            position: Duration::ZERO,
            duration: None,
            volume: 1.0,
            muted: false,
        }
    }

    pub fn progress(&self) -> Option<f64> {
        self.duration
            .filter(|d| !d.is_zero())
            .map(|d| self.position.as_secs_f64() / d.as_secs_f64())
    }
}

/// Media library — indexed collection of media items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub items: Vec<MediaItem>,
    pub playlists: Vec<Playlist>,
    pub scan_paths: Vec<PathBuf>,
}

impl Library {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            playlists: Vec::new(),
            scan_paths: Vec::new(),
        }
    }

    pub fn add_item(&mut self, item: MediaItem) -> Uuid {
        let id = item.id;
        self.items.push(item);
        id
    }

    pub fn find_by_id(&self, id: Uuid) -> Option<&MediaItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn find_by_id_mut(&mut self, id: Uuid) -> Option<&mut MediaItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    pub fn find_by_path(&self, path: &Path) -> Option<&MediaItem> {
        self.items.iter().find(|i| i.path == path)
    }

    pub fn remove(&mut self, id: Uuid) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.id != id);
        // Also remove from all playlists
        for playlist in &mut self.playlists {
            playlist.remove(id);
        }
        self.items.len() != before
    }

    pub fn search(&self, query: &str) -> Vec<&MediaItem> {
        let q = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&q)
                    || item
                        .artist
                        .as_ref()
                        .is_some_and(|a| a.to_lowercase().contains(&q))
                    || item
                        .album
                        .as_ref()
                        .is_some_and(|a| a.to_lowercase().contains(&q))
                    || item.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    pub fn audio_items(&self) -> Vec<&MediaItem> {
        self.items.iter().filter(|i| i.is_audio()).collect()
    }

    pub fn video_items(&self) -> Vec<&MediaItem> {
        self.items.iter().filter(|i| i.is_video()).collect()
    }

    pub fn add_scan_path(&mut self, path: PathBuf) {
        if !self.scan_paths.contains(&path) {
            self.scan_paths.push(path);
        }
    }

    pub fn create_playlist(&mut self, name: impl Into<String>) -> Uuid {
        let playlist = Playlist::new(name);
        let id = playlist.id;
        self.playlists.push(playlist);
        id
    }

    pub fn find_playlist(&self, id: Uuid) -> Option<&Playlist> {
        self.playlists.iter().find(|p| p.id == id)
    }

    pub fn find_playlist_mut(&mut self, id: Uuid) -> Option<&mut Playlist> {
        self.playlists.iter_mut().find(|p| p.id == id)
    }
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

/// Jalwa error types
#[derive(Debug, thiserror::Error)]
pub enum JalwaError {
    #[error("media not found: {0}")]
    NotFound(String),
    #[error("playback error: {0}")]
    Playback(String),
    #[error("library error: {0}")]
    Library(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("scanner error: {0}")]
    Scanner(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("tarang error: {0}")]
    Tarang(#[from] tarang_core::TarangError),
}

pub type Result<T> = std::result::Result<T, JalwaError>;

/// Shared test fixtures for use across the workspace.
///
/// Always compiled so downstream crate tests can use it.
/// The functions are cheap (no I/O, no allocations beyond the returned struct).
pub mod test_fixtures {
    use super::*;
    use tarang_core::{AudioCodec, ContainerFormat};

    /// Create a `MediaItem` with common test defaults.
    pub fn make_media_item(title: &str, artist: &str, duration_secs: u64) -> MediaItem {
        MediaItem {
            id: Uuid::new_v4(),
            path: std::path::PathBuf::from(format!("/music/{title}.flac")),
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: Some("Album".to_string()),
            duration: Some(std::time::Duration::from_secs(duration_secs)),
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

    /// Generate a minimal valid WAV file in memory (mono 16-bit PCM, 440 Hz sine).
    pub fn make_test_wav(num_samples: u32, sample_rate: u32) -> Vec<u8> {
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
        buf.extend_from_slice(&1u16.to_le_bytes());
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
            buf.extend_from_slice(&((s * 16000.0) as i16).to_le_bytes());
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tarang_core::*;

    fn make_audio_info() -> MediaInfo {
        MediaInfo {
            id: Uuid::new_v4(),
            format: ContainerFormat::Flac,
            streams: vec![StreamInfo::Audio(AudioStreamInfo {
                codec: AudioCodec::Flac,
                sample_rate: 44100,
                channels: 2,
                sample_format: SampleFormat::I16,
                bitrate: None,
                duration: Some(Duration::from_secs(240)),
            })],
            duration: Some(Duration::from_secs(240)),
            file_size: Some(30_000_000),
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
        }
    }

    fn make_video_info() -> MediaInfo {
        MediaInfo {
            id: Uuid::new_v4(),
            format: ContainerFormat::Mp4,
            streams: vec![
                StreamInfo::Video(VideoStreamInfo {
                    codec: VideoCodec::H264,
                    width: 1920,
                    height: 1080,
                    pixel_format: PixelFormat::Yuv420p,
                    frame_rate: 24.0,
                    bitrate: Some(5_000_000),
                    duration: Some(Duration::from_secs(7200)),
                }),
                StreamInfo::Audio(AudioStreamInfo {
                    codec: AudioCodec::Aac,
                    sample_rate: 48000,
                    channels: 2,
                    sample_format: SampleFormat::F32,
                    bitrate: Some(128_000),
                    duration: Some(Duration::from_secs(7200)),
                }),
            ],
            duration: Some(Duration::from_secs(7200)),
            file_size: Some(4_500_000_000),
            title: Some("Test Movie".to_string()),
            artist: None,
            album: None,
        }
    }

    // MediaItem tests

    #[test]
    fn media_item_from_audio_probe() {
        let info = make_audio_info();
        let item = MediaItem::from_probe(PathBuf::from("/music/song.flac"), &info);
        assert!(item.is_audio());
        assert!(!item.is_video());
        assert_eq!(item.title, "song");
        assert_eq!(item.artist, Some("Test Artist".to_string()));
        assert_eq!(item.audio_codec, Some(AudioCodec::Flac));
        assert_eq!(item.video_codec, None);
        assert_eq!(item.play_count, 0);
    }

    #[test]
    fn media_item_from_video_probe() {
        let info = make_video_info();
        let item = MediaItem::from_probe(PathBuf::from("/videos/movie.mp4"), &info);
        assert!(item.is_video());
        assert!(!item.is_audio());
        assert_eq!(item.audio_codec, Some(AudioCodec::Aac));
        assert_eq!(item.video_codec, Some(VideoCodec::H264));
    }

    #[test]
    fn media_type_display() {
        assert_eq!(MediaType::Audio.to_string(), "audio");
        assert_eq!(MediaType::Video.to_string(), "video");
    }

    // Playlist tests

    #[test]
    fn playlist_new() {
        let pl = Playlist::new("My Playlist");
        assert_eq!(pl.name, "My Playlist");
        assert!(pl.is_empty());
        assert_eq!(pl.len(), 0);
    }

    #[test]
    fn playlist_add_remove() {
        let mut pl = Playlist::new("Test");
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        pl.add(id1);
        pl.add(id2);
        assert_eq!(pl.len(), 2);

        assert!(pl.remove(id1));
        assert_eq!(pl.len(), 1);
        assert!(!pl.remove(id1)); // already removed
    }

    #[test]
    fn playlist_clear() {
        let mut pl = Playlist::new("Test");
        pl.add(Uuid::new_v4());
        pl.add(Uuid::new_v4());
        pl.clear();
        assert!(pl.is_empty());
    }

    // PlayQueue tests

    #[test]
    fn queue_new() {
        let q = PlayQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.current(), None);
    }

    #[test]
    fn queue_enqueue() {
        let mut q = PlayQueue::new();
        let id = Uuid::new_v4();
        q.enqueue(id);
        assert_eq!(q.len(), 1);
        assert_eq!(q.current(), Some(id));
    }

    #[test]
    fn queue_enqueue_many() {
        let mut q = PlayQueue::new();
        let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        q.enqueue_many(ids.clone());
        assert_eq!(q.len(), 5);
        assert_eq!(q.current(), Some(ids[0]));
    }

    #[test]
    fn queue_next() {
        let mut q = PlayQueue::new();
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        q.enqueue_many(ids.clone());

        assert_eq!(q.current(), Some(ids[0]));
        assert_eq!(q.advance(), Some(ids[1]));
        assert_eq!(q.current(), Some(ids[1]));
        assert_eq!(q.advance(), Some(ids[2]));
        assert_eq!(q.advance(), None); // end of queue
    }

    #[test]
    fn queue_previous() {
        let mut q = PlayQueue::new();
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        q.enqueue_many(ids.clone());

        q.advance(); // -> ids[1]
        q.advance(); // -> ids[2]
        assert_eq!(q.go_back(), Some(ids[1]));
        assert_eq!(q.go_back(), Some(ids[0]));
        assert_eq!(q.go_back(), None); // at start
    }

    #[test]
    fn queue_repeat_all() {
        let mut q = PlayQueue::new();
        q.repeat_mode = RepeatMode::All;
        let ids: Vec<Uuid> = (0..2).map(|_| Uuid::new_v4()).collect();
        q.enqueue_many(ids.clone());

        q.advance(); // -> ids[1]
        assert_eq!(q.advance(), Some(ids[0])); // wraps around
    }

    #[test]
    fn queue_repeat_all_previous() {
        let mut q = PlayQueue::new();
        q.repeat_mode = RepeatMode::All;
        let ids: Vec<Uuid> = (0..2).map(|_| Uuid::new_v4()).collect();
        q.enqueue_many(ids.clone());

        assert_eq!(q.go_back(), Some(ids[1])); // wraps backward
    }

    #[test]
    fn queue_clear() {
        let mut q = PlayQueue::new();
        q.enqueue(Uuid::new_v4());
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.current(), None);
    }

    // RepeatMode tests

    #[test]
    fn repeat_mode_cycle() {
        assert_eq!(RepeatMode::Off.cycle(), RepeatMode::One);
        assert_eq!(RepeatMode::One.cycle(), RepeatMode::All);
        assert_eq!(RepeatMode::All.cycle(), RepeatMode::Off);
    }

    #[test]
    fn repeat_mode_display() {
        assert_eq!(RepeatMode::Off.to_string(), "off");
        assert_eq!(RepeatMode::One.to_string(), "one");
        assert_eq!(RepeatMode::All.to_string(), "all");
    }

    // PlaybackState tests

    #[test]
    fn playback_state_display() {
        assert_eq!(PlaybackState::Stopped.to_string(), "stopped");
        assert_eq!(PlaybackState::Playing.to_string(), "playing");
        assert_eq!(PlaybackState::Paused.to_string(), "paused");
        assert_eq!(PlaybackState::Buffering.to_string(), "buffering");
    }

    // PlaybackStatus tests

    #[test]
    fn playback_status_stopped() {
        let status = PlaybackStatus::stopped();
        assert_eq!(status.state, PlaybackState::Stopped);
        assert_eq!(status.volume, 1.0);
        assert!(!status.muted);
        assert_eq!(status.progress(), None);
    }

    #[test]
    fn playback_status_progress() {
        let status = PlaybackStatus {
            state: PlaybackState::Playing,
            current_item: Some(Uuid::new_v4()),
            position: Duration::from_secs(60),
            duration: Some(Duration::from_secs(240)),
            volume: 0.8,
            muted: false,
        };
        let progress = status.progress().unwrap();
        assert!((progress - 0.25).abs() < 0.001);
    }

    #[test]
    fn playback_status_progress_zero_duration() {
        let status = PlaybackStatus {
            state: PlaybackState::Playing,
            current_item: Some(Uuid::new_v4()),
            position: Duration::from_secs(10),
            duration: Some(Duration::ZERO),
            volume: 1.0,
            muted: false,
        };
        assert_eq!(status.progress(), None);
    }

    // Library tests

    #[test]
    fn library_new() {
        let lib = Library::new();
        assert!(lib.items.is_empty());
        assert!(lib.playlists.is_empty());
    }

    #[test]
    fn library_add_find() {
        let mut lib = Library::new();
        let info = make_audio_info();
        let item = MediaItem::from_probe(PathBuf::from("/music/song.flac"), &info);
        let id = item.id;
        lib.add_item(item);

        assert!(lib.find_by_id(id).is_some());
        assert!(lib.find_by_path(Path::new("/music/song.flac")).is_some());
        assert!(lib.find_by_id(Uuid::new_v4()).is_none());
    }

    #[test]
    fn library_remove() {
        let mut lib = Library::new();
        let info = make_audio_info();
        let item = MediaItem::from_probe(PathBuf::from("/music/song.flac"), &info);
        let id = item.id;
        lib.add_item(item);

        // Also add to a playlist
        let pl_id = lib.create_playlist("Test PL");
        lib.find_playlist_mut(pl_id).unwrap().add(id);

        assert!(lib.remove(id));
        assert!(lib.find_by_id(id).is_none());
        // Should also be removed from playlist
        assert!(lib.find_playlist(pl_id).unwrap().is_empty());
    }

    #[test]
    fn library_search() {
        let mut lib = Library::new();
        let info = make_audio_info();
        let mut item = MediaItem::from_probe(PathBuf::from("/music/song.flac"), &info);
        item.title = "Bohemian Rhapsody".to_string();
        item.artist = Some("Queen".to_string());
        lib.add_item(item);

        assert_eq!(lib.search("bohemian").len(), 1);
        assert_eq!(lib.search("queen").len(), 1);
        assert_eq!(lib.search("RHAPSODY").len(), 1);
        assert_eq!(lib.search("nonexistent").len(), 0);
    }

    #[test]
    fn library_search_tags() {
        let mut lib = Library::new();
        let info = make_audio_info();
        let mut item = MediaItem::from_probe(PathBuf::from("/music/song.flac"), &info);
        item.tags = vec!["rock".to_string(), "classic".to_string()];
        lib.add_item(item);

        assert_eq!(lib.search("rock").len(), 1);
        assert_eq!(lib.search("classic").len(), 1);
    }

    #[test]
    fn library_filter_by_type() {
        let mut lib = Library::new();
        let audio_info = make_audio_info();
        let video_info = make_video_info();
        lib.add_item(MediaItem::from_probe(
            PathBuf::from("/music/song.flac"),
            &audio_info,
        ));
        lib.add_item(MediaItem::from_probe(
            PathBuf::from("/videos/movie.mp4"),
            &video_info,
        ));

        assert_eq!(lib.audio_items().len(), 1);
        assert_eq!(lib.video_items().len(), 1);
    }

    #[test]
    fn library_scan_paths() {
        let mut lib = Library::new();
        lib.add_scan_path(PathBuf::from("/home/user/Music"));
        lib.add_scan_path(PathBuf::from("/home/user/Music")); // duplicate
        assert_eq!(lib.scan_paths.len(), 1);
        lib.add_scan_path(PathBuf::from("/home/user/Videos"));
        assert_eq!(lib.scan_paths.len(), 2);
    }

    #[test]
    fn library_playlists() {
        let mut lib = Library::new();
        let pl_id = lib.create_playlist("Favorites");
        let pl = lib.find_playlist(pl_id).unwrap();
        assert_eq!(pl.name, "Favorites");
        assert!(pl.is_empty());
    }

    // Error tests

    #[test]
    fn error_display() {
        let err = JalwaError::NotFound("song.mp3".to_string());
        assert_eq!(err.to_string(), "media not found: song.mp3");

        let err = JalwaError::Playback("buffer underrun".to_string());
        assert_eq!(err.to_string(), "playback error: buffer underrun");
    }

    // Serialization tests

    #[test]
    fn media_type_serialization() {
        let json = serde_json::to_string(&MediaType::Audio).unwrap();
        let parsed: MediaType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MediaType::Audio);
    }

    #[test]
    fn playback_state_serialization() {
        let json = serde_json::to_string(&PlaybackState::Playing).unwrap();
        let parsed: PlaybackState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PlaybackState::Playing);
    }

    #[test]
    fn repeat_mode_serialization() {
        let json = serde_json::to_string(&RepeatMode::All).unwrap();
        let parsed: RepeatMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, RepeatMode::All);
    }
}
