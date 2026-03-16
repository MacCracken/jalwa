//! jalwa-playback — Playback engine for the Jalwa media player
//!
//! Manages the decode pipeline (via tarang) and audio output (PipeWire).
//! Handles play, pause, seek, volume, and track switching.

pub mod decode_thread;

use jalwa_core::{JalwaError, PlaybackState, PlaybackStatus, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

pub use decode_thread::{DecodeStatus, EngineCommand, EngineEvent};

/// Playback engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub buffer_size: usize,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            buffer_size: 4096,
            sample_rate: 48000,
            channels: 2,
        }
    }
}

/// The playback engine — orchestrates tarang decode + audio output
pub struct PlaybackEngine {
    config: EngineConfig,
    state: PlaybackState,
    current_path: Option<PathBuf>,
    position: Duration,
    duration: Option<Duration>,
    volume: f32,
    muted: bool,
    // Channel-based communication with decode thread
    cmd_tx: Option<mpsc::Sender<EngineCommand>>,
    decode_status: Option<Arc<Mutex<DecodeStatus>>>,
    event_rx: Option<mpsc::Receiver<EngineEvent>>,
    decode_handle: Option<std::thread::JoinHandle<()>>,
}

impl PlaybackEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            state: PlaybackState::Stopped,
            current_path: None,
            position: Duration::ZERO,
            duration: None,
            volume: 1.0,
            muted: false,
            cmd_tx: None,
            decode_status: None,
            event_rx: None,
            decode_handle: None,
        }
    }

    /// Open a media file for playback (probe only)
    pub fn open(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(JalwaError::NotFound(path.to_string_lossy().to_string()));
        }

        // Stop any existing playback
        self.stop();

        // Probe the file via tarang
        let file = std::fs::File::open(path)?;
        let info = tarang_audio::probe_audio(file).map_err(JalwaError::Tarang)?;

        self.duration = info.duration;
        self.current_path = Some(path.to_path_buf());
        self.position = Duration::ZERO;
        self.state = PlaybackState::Stopped;

        tracing::info!(
            path = %path.display(),
            format = %info.format,
            streams = info.streams.len(),
            "opened media file"
        );

        Ok(())
    }

    /// Start or resume playback
    pub fn play(&mut self) -> Result<()> {
        let path = self.current_path.clone().ok_or_else(|| {
            JalwaError::Playback("no file loaded".to_string())
        })?;

        if self.state == PlaybackState::Paused {
            // Resume existing decode thread
            if let Some(ref tx) = self.cmd_tx {
                let _ = tx.send(EngineCommand::Resume);
            }
            self.state = PlaybackState::Playing;
            return Ok(());
        }

        // Spawn new decode thread
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DecodeStatus::default()));
        let status_clone = status.clone();
        let (event_tx, event_rx) = mpsc::channel();

        let config = self.config.clone();
        let duration = self.duration;

        let handle = std::thread::Builder::new()
            .name("jalwa-decode".into())
            .spawn(move || {
                decode_thread::decode_loop(path, cmd_rx, status_clone, event_tx, config, duration);
            })
            .map_err(|e| JalwaError::Playback(format!("spawn decode thread: {e}")))?;

        self.cmd_tx = Some(cmd_tx);
        self.decode_status = Some(status);
        self.event_rx = Some(event_rx);
        self.decode_handle = Some(handle);
        self.state = PlaybackState::Playing;

        Ok(())
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            if let Some(ref tx) = self.cmd_tx {
                let _ = tx.send(EngineCommand::Pause);
            }
            self.state = PlaybackState::Paused;
        }
    }

    /// Stop playback and reset position
    pub fn stop(&mut self) {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(EngineCommand::Stop);
        }
        // Wait for decode thread to finish
        if let Some(handle) = self.decode_handle.take() {
            let _ = handle.join();
        }
        self.cmd_tx = None;
        self.decode_status = None;
        self.event_rx = None;
        self.state = PlaybackState::Stopped;
        self.position = Duration::ZERO;
    }

    /// Toggle play/pause
    pub fn toggle(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Playing => {
                self.pause();
                Ok(())
            }
            PlaybackState::Paused | PlaybackState::Stopped => self.play(),
            PlaybackState::Buffering => Ok(()),
        }
    }

    /// Seek to a position
    pub fn seek(&mut self, position: Duration) -> Result<()> {
        if self.duration.is_some_and(|dur| position > dur) {
            return Err(JalwaError::Playback("seek beyond end".to_string()));
        }
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(EngineCommand::Seek(position));
        }
        self.position = position;
        Ok(())
    }

    /// Seek by a relative offset (can be negative)
    pub fn seek_relative(&mut self, offset_secs: f64) -> Result<()> {
        let current = self.position().as_secs_f64();
        let new_pos = (current + offset_secs).max(0.0);
        let target = if let Some(dur) = self.duration {
            Duration::from_secs_f64(new_pos.min(dur.as_secs_f64()))
        } else {
            Duration::from_secs_f64(new_pos)
        };
        self.seek(target)
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(EngineCommand::Volume(self.volume));
        }
    }

    /// Get current volume
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Toggle mute
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(EngineCommand::Mute(self.muted));
        }
    }

    /// Get mute state
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Get current playback state
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Get current position (reads from decode thread if running)
    pub fn position(&self) -> Duration {
        if let Some(ref status) = self.decode_status {
            if let Ok(s) = status.lock() {
                return s.position;
            }
        }
        self.position
    }

    /// Get current file's duration
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    /// Get current file path
    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    /// Get full playback status snapshot
    pub fn status(&self) -> PlaybackStatus {
        let position = self.position();
        PlaybackStatus {
            state: self.state,
            current_item: None, // Caller maps path -> UUID
            position,
            duration: self.duration,
            volume: self.volume,
            muted: self.muted,
        }
    }

    /// Get engine config
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Get a reference to the event receiver for polling events
    pub fn events(&self) -> Option<&mpsc::Receiver<EngineEvent>> {
        self.event_rx.as_ref()
    }

    /// Prepare next track for gapless playback
    pub fn prepare_next(&self, path: &Path) {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(EngineCommand::PrepareNext {
                path: path.to_path_buf(),
            });
        }
    }

    /// Poll and process engine events, updating internal state.
    /// Returns collected events for the caller to handle.
    pub fn poll_events(&mut self) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        if let Some(ref rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match &event {
                    EngineEvent::StateChanged(s) => self.state = *s,
                    EngineEvent::TrackFinished => {
                        self.state = PlaybackState::Stopped;
                    }
                    _ => {}
                }
                events.push(event);
            }
        }
        // Update position from decode status
        if let Some(ref status) = self.decode_status {
            if let Ok(s) = status.lock() {
                self.position = s.position;
            }
        }
        events
    }
}

impl Drop for PlaybackEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Format a duration as HH:MM:SS or MM:SS
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_default() {
        let config = EngineConfig::default();
        assert_eq!(config.buffer_size, 4096);
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn engine_new() {
        let engine = PlaybackEngine::new(EngineConfig::default());
        assert_eq!(engine.state(), PlaybackState::Stopped);
        assert_eq!(engine.volume(), 1.0);
        assert!(!engine.muted());
        assert_eq!(engine.position(), Duration::ZERO);
        assert!(engine.current_path().is_none());
    }

    #[test]
    fn play_without_file() {
        let mut engine = PlaybackEngine::new(EngineConfig::default());
        assert!(engine.play().is_err());
    }

    #[test]
    fn open_nonexistent() {
        let mut engine = PlaybackEngine::new(EngineConfig::default());
        assert!(engine.open(Path::new("/nonexistent/file.mp3")).is_err());
    }

    #[test]
    fn volume_clamp() {
        let mut engine = PlaybackEngine::new(EngineConfig::default());
        engine.set_volume(1.5);
        assert_eq!(engine.volume(), 1.0);
        engine.set_volume(-0.5);
        assert_eq!(engine.volume(), 0.0);
        engine.set_volume(0.7);
        assert_eq!(engine.volume(), 0.7);
    }

    #[test]
    fn mute_toggle() {
        let mut engine = PlaybackEngine::new(EngineConfig::default());
        assert!(!engine.muted());
        engine.toggle_mute();
        assert!(engine.muted());
        engine.toggle_mute();
        assert!(!engine.muted());
    }

    #[test]
    fn status_stopped() {
        let engine = PlaybackEngine::new(EngineConfig::default());
        let status = engine.status();
        assert_eq!(status.state, PlaybackState::Stopped);
        assert_eq!(status.volume, 1.0);
        assert_eq!(status.progress(), None);
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(Duration::from_secs(65)), "1:05");
        assert_eq!(format_duration(Duration::from_secs(599)), "9:59");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1:01:01");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2:00:00");
    }
}
