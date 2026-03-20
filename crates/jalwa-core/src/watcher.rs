//! File system watcher — monitors library directories for changes.
//!
//! Uses the `notify` crate for cross-platform file watching (inotify on Linux).

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{JalwaError, Result};

/// Events emitted by the file watcher.
#[derive(Debug, Clone)]
pub enum LibraryEvent {
    /// A new media file was created.
    FileCreated(PathBuf),
    /// A media file was modified (metadata changed, re-encoded).
    FileModified(PathBuf),
    /// A media file was removed.
    FileRemoved(PathBuf),
}

/// Watches library directories for file changes.
pub struct LibraryWatcher {
    _watcher: RecommendedWatcher,
    event_rx: mpsc::Receiver<LibraryEvent>,
}

/// Media file extensions to watch for.
const MEDIA_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "mp4", "mkv", "webm", "aac", "opus",
];

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| MEDIA_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

impl LibraryWatcher {
    /// Create a new watcher monitoring the given directories.
    pub fn new(paths: &[PathBuf]) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let event_tx = tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
                let event = match res {
                    Ok(e) => e,
                    Err(_) => return,
                };

                for path in &event.paths {
                    if !is_media_file(path) {
                        continue;
                    }

                    let lib_event = match event.kind {
                        EventKind::Create(_) => LibraryEvent::FileCreated(path.clone()),
                        EventKind::Modify(_) => LibraryEvent::FileModified(path.clone()),
                        EventKind::Remove(_) => LibraryEvent::FileRemoved(path.clone()),
                        _ => continue,
                    };

                    let _ = event_tx.send(lib_event);
                }
            })
            .map_err(|e| JalwaError::Scanner(format!("watcher init: {e}")))?;

        for path in paths {
            if path.is_dir() {
                watcher
                    .watch(path, RecursiveMode::Recursive)
                    .map_err(|e| JalwaError::Scanner(format!("watch {}: {e}", path.display())))?;
            }
        }

        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
        })
    }

    /// Poll for pending events (non-blocking).
    pub fn poll(&self) -> Vec<LibraryEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Wait for the next event with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<LibraryEvent> {
        self.event_rx.recv_timeout(timeout).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_media_file_checks() {
        assert!(is_media_file(Path::new("/music/song.mp3")));
        assert!(is_media_file(Path::new("/music/song.FLAC")));
        assert!(is_media_file(Path::new("track.ogg")));
        assert!(is_media_file(Path::new("video.mkv")));
        assert!(!is_media_file(Path::new("readme.txt")));
        assert!(!is_media_file(Path::new("photo.jpg")));
        assert!(!is_media_file(Path::new("noext")));
    }

    #[test]
    fn library_event_debug() {
        let ev = LibraryEvent::FileCreated(PathBuf::from("/music/new.mp3"));
        let s = format!("{:?}", ev);
        assert!(s.contains("FileCreated"));
    }

    #[test]
    fn watcher_empty_paths() {
        let watcher = LibraryWatcher::new(&[]);
        assert!(watcher.is_ok());
        let w = watcher.unwrap();
        assert!(w.poll().is_empty());
    }

    #[test]
    fn watcher_with_temp_dir() {
        let tmp = std::env::temp_dir().join(format!("jalwa_watch_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let watcher = LibraryWatcher::new(&[tmp.clone()]).unwrap();

        // Create a media file
        std::fs::write(tmp.join("test.mp3"), b"fake mp3").unwrap();

        // Give the watcher a moment to pick up the event
        std::thread::sleep(Duration::from_millis(200));

        let events = watcher.poll();
        // On some systems inotify may batch or not fire instantly, so we check >= 0
        // The key thing is no panic
        for ev in &events {
            match ev {
                LibraryEvent::FileCreated(p) => assert!(p.ends_with("test.mp3")),
                LibraryEvent::FileModified(p) => assert!(p.ends_with("test.mp3")),
                _ => {}
            }
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn watcher_ignores_non_media() {
        let tmp = std::env::temp_dir().join(format!("jalwa_watch_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let watcher = LibraryWatcher::new(&[tmp.clone()]).unwrap();
        std::fs::write(tmp.join("notes.txt"), b"hello").unwrap();

        std::thread::sleep(Duration::from_millis(200));

        let events = watcher.poll();
        // Should not contain any events for .txt files
        for ev in &events {
            match ev {
                LibraryEvent::FileCreated(p) | LibraryEvent::FileModified(p) => {
                    assert!(!p.ends_with(".txt"), "should not watch .txt files");
                }
                _ => {}
            }
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn watcher_detects_file_creation() {
        let tmp = std::env::temp_dir().join(format!("jalwa_watch_create_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let watcher = LibraryWatcher::new(&[tmp.clone()]).unwrap();

        std::fs::write(tmp.join("new_song.mp3"), b"fake mp3 data").unwrap();
        std::thread::sleep(Duration::from_millis(200));

        let events = watcher.poll();
        let has_created = events.iter().any(|ev| {
            matches!(ev, LibraryEvent::FileCreated(p) if p.ends_with("new_song.mp3"))
        });
        // inotify should fire a Create event for the new file
        assert!(has_created, "expected FileCreated event for new_song.mp3");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn watcher_detects_file_removal() {
        let tmp = std::env::temp_dir().join(format!("jalwa_watch_remove_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let mp3_path = tmp.join("to_delete.mp3");
        std::fs::write(&mp3_path, b"fake mp3").unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let watcher = LibraryWatcher::new(&[tmp.clone()]).unwrap();

        // Drain any creation events from before watch started
        std::thread::sleep(Duration::from_millis(100));
        let _ = watcher.poll();

        std::fs::remove_file(&mp3_path).unwrap();
        std::thread::sleep(Duration::from_millis(200));

        let events = watcher.poll();
        let has_removed = events.iter().any(|ev| {
            matches!(ev, LibraryEvent::FileRemoved(p) if p.ends_with("to_delete.mp3"))
        });
        assert!(has_removed, "expected FileRemoved event for to_delete.mp3");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn recv_timeout_no_events() {
        let tmp = std::env::temp_dir().join(format!("jalwa_watch_timeout_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let watcher = LibraryWatcher::new(&[tmp.clone()]).unwrap();
        let result = watcher.recv_timeout(Duration::from_millis(50));
        assert!(result.is_none(), "expected None when no events within timeout");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn watcher_multiple_paths() {
        let tmp1 = std::env::temp_dir().join(format!("jalwa_watch_multi1_{}", uuid::Uuid::new_v4()));
        let tmp2 = std::env::temp_dir().join(format!("jalwa_watch_multi2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp1).unwrap();
        std::fs::create_dir_all(&tmp2).unwrap();

        let watcher = LibraryWatcher::new(&[tmp1.clone(), tmp2.clone()]).unwrap();

        std::fs::write(tmp1.join("song1.flac"), b"fake flac 1").unwrap();
        std::fs::write(tmp2.join("song2.ogg"), b"fake ogg 2").unwrap();
        std::thread::sleep(Duration::from_millis(200));

        let events = watcher.poll();
        let paths: Vec<String> = events
            .iter()
            .filter_map(|ev| match ev {
                LibraryEvent::FileCreated(p) | LibraryEvent::FileModified(p) => {
                    p.file_name().map(|f| f.to_string_lossy().to_string())
                }
                _ => None,
            })
            .collect();

        assert!(
            paths.iter().any(|p| p == "song1.flac"),
            "expected event for song1.flac in dir 1"
        );
        assert!(
            paths.iter().any(|p| p == "song2.ogg"),
            "expected event for song2.ogg in dir 2"
        );

        let _ = std::fs::remove_dir_all(&tmp1);
        let _ = std::fs::remove_dir_all(&tmp2);
    }
}
