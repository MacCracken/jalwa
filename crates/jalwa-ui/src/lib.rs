//! jalwa-ui — UI layer for the Jalwa media player
//!
//! Terminal UI (TUI) for initial MVP, desktop GUI planned.
//! Renders playback status, library browser, and queue.

use jalwa_core::{Library, MediaItem, PlayQueue, PlaybackState, PlaybackStatus, RepeatMode};
use jalwa_playback::format_duration;
use std::time::Duration;

/// Render a playback status bar as a string
pub fn render_status_bar(status: &PlaybackStatus, item: Option<&MediaItem>) -> String {
    let state_icon = match status.state {
        PlaybackState::Playing => ">>",
        PlaybackState::Paused => "||",
        PlaybackState::Stopped => "[]",
        PlaybackState::Buffering => "..",
    };

    let title = item.map(|i| i.title.as_str()).unwrap_or("No media loaded");

    let artist = item.and_then(|i| i.artist.as_deref()).unwrap_or("");

    let position = format_duration(status.position);
    let duration = status
        .duration
        .map(format_duration)
        .unwrap_or_else(|| "--:--".to_string());

    let volume = if status.muted {
        "MUTE".to_string()
    } else {
        format!("{}%", (status.volume * 100.0) as u8)
    };

    if artist.is_empty() {
        format!("{state_icon} {title}  [{position} / {duration}]  Vol: {volume}")
    } else {
        format!("{state_icon} {artist} - {title}  [{position} / {duration}]  Vol: {volume}")
    }
}

/// Render a progress bar
pub fn render_progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "=".repeat(filled.min(width)), " ".repeat(empty))
}

/// Render the play queue summary
pub fn render_queue_summary(queue: &PlayQueue) -> String {
    let pos = queue
        .position
        .map(|p| format!("{}/{}", p + 1, queue.len()))
        .unwrap_or_else(|| format!("0/{}", queue.len()));

    let repeat = match queue.repeat_mode {
        RepeatMode::Off => "",
        RepeatMode::One => " [R1]",
        RepeatMode::All => " [RA]",
    };

    let shuffle = if queue.shuffle { " [S]" } else { "" };

    format!("Queue: {pos}{repeat}{shuffle}")
}

/// Render a library item as a single line
pub fn render_library_item(item: &MediaItem, index: usize) -> String {
    let duration = item
        .duration
        .map(format_duration)
        .unwrap_or_else(|| "?:??".to_string());

    let artist = item.artist.as_deref().unwrap_or("Unknown");

    format!(
        "{:>3}. {} - {} [{}] ({})",
        index + 1,
        artist,
        item.title,
        duration,
        item.media_type
    )
}

/// Render library stats
pub fn render_library_stats(library: &Library) -> String {
    let total = library.items.len();
    let audio = library.audio_items().len();
    let video = library.video_items().len();
    let playlists = library.playlists.len();

    let total_duration: Duration = library.items.iter().filter_map(|i| i.duration).sum();

    format!(
        "Library: {} items ({} audio, {} video), {} playlists, total: {}",
        total,
        audio,
        video,
        playlists,
        format_duration(total_duration)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::*;
    use std::path::PathBuf;
    use tarang_core::*;
    use uuid::Uuid;

    fn make_item(title: &str, artist: &str, duration_secs: u64) -> MediaItem {
        MediaItem {
            id: Uuid::new_v4(),
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: None,
            duration: Some(Duration::from_secs(duration_secs)),
            format: ContainerFormat::Flac,
            audio_codec: Some(AudioCodec::Flac),
            video_codec: None,
            media_type: MediaType::Audio,
            added_at: chrono::Utc::now(),
            last_played: None,
            play_count: 0,
            rating: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn status_bar_playing() {
        let status = PlaybackStatus {
            state: PlaybackState::Playing,
            current_item: Some(Uuid::new_v4()),
            position: Duration::from_secs(65),
            duration: Some(Duration::from_secs(240)),
            volume: 0.8,
            muted: false,
        };
        let item = make_item("Song", "Artist", 240);
        let bar = render_status_bar(&status, Some(&item));
        assert!(bar.contains(">>"));
        assert!(bar.contains("Artist - Song"));
        assert!(bar.contains("1:05"));
        assert!(bar.contains("4:00"));
        assert!(bar.contains("80%"));
    }

    #[test]
    fn status_bar_stopped_no_item() {
        let status = PlaybackStatus::stopped();
        let bar = render_status_bar(&status, None);
        assert!(bar.contains("[]"));
        assert!(bar.contains("No media loaded"));
    }

    #[test]
    fn status_bar_muted() {
        let status = PlaybackStatus {
            state: PlaybackState::Paused,
            current_item: None,
            position: Duration::ZERO,
            duration: None,
            volume: 0.5,
            muted: true,
        };
        let bar = render_status_bar(&status, None);
        assert!(bar.contains("MUTE"));
    }

    #[test]
    fn progress_bar() {
        let bar = render_progress_bar(0.5, 20);
        assert_eq!(bar.len(), 22); // 20 + 2 brackets
        assert!(bar.starts_with('['));
        assert!(bar.ends_with(']'));
        assert_eq!(bar.matches('=').count(), 10);
    }

    #[test]
    fn progress_bar_empty() {
        let bar = render_progress_bar(0.0, 10);
        assert_eq!(bar, "[          ]");
    }

    #[test]
    fn progress_bar_full() {
        let bar = render_progress_bar(1.0, 10);
        assert_eq!(bar, "[==========]");
    }

    #[test]
    fn queue_summary_empty() {
        let q = PlayQueue::new();
        let summary = render_queue_summary(&q);
        assert_eq!(summary, "Queue: 0/0");
    }

    #[test]
    fn queue_summary_with_items() {
        let mut q = PlayQueue::new();
        q.enqueue(Uuid::new_v4());
        q.enqueue(Uuid::new_v4());
        let summary = render_queue_summary(&q);
        assert_eq!(summary, "Queue: 1/2");
    }

    #[test]
    fn queue_summary_repeat() {
        let mut q = PlayQueue::new();
        q.enqueue(Uuid::new_v4());
        q.repeat_mode = RepeatMode::All;
        let summary = render_queue_summary(&q);
        assert!(summary.contains("[RA]"));
    }

    #[test]
    fn queue_summary_shuffle() {
        let mut q = PlayQueue::new();
        q.enqueue(Uuid::new_v4());
        q.shuffle = true;
        let summary = render_queue_summary(&q);
        assert!(summary.contains("[S]"));
    }

    #[test]
    fn library_item_render() {
        let item = make_item("Bohemian Rhapsody", "Queen", 354);
        let line = render_library_item(&item, 0);
        assert!(line.contains("1."));
        assert!(line.contains("Queen"));
        assert!(line.contains("Bohemian Rhapsody"));
        assert!(line.contains("5:54"));
        assert!(line.contains("audio"));
    }

    #[test]
    fn library_stats() {
        let mut lib = Library::new();
        lib.add_item(make_item("Song 1", "A", 180));
        lib.add_item(make_item("Song 2", "B", 240));
        lib.create_playlist("Favs");

        let stats = render_library_stats(&lib);
        assert!(stats.contains("2 items"));
        assert!(stats.contains("2 audio"));
        assert!(stats.contains("0 video"));
        assert!(stats.contains("1 playlists"));
        assert!(stats.contains("7:00")); // 180 + 240 = 420s = 7:00
    }
}
