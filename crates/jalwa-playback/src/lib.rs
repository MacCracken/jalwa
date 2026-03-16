//! jalwa-playback — Playback engine for the Jalwa media player
//!
//! Manages the decode pipeline (via tarang) and audio output (PipeWire).
//! Handles play, pause, seek, volume, and track switching.

use jalwa_core::{JalwaError, PlaybackState, PlaybackStatus, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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
    playing: Arc<AtomicBool>,
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
            playing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Open a media file for playback
    pub fn open(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(JalwaError::NotFound(path.to_string_lossy().to_string()));
        }

        // Probe the file via tarang
        let file = std::fs::File::open(path)?;
        let info = tarang_audio::probe_audio(file).map_err(JalwaError::Tarang)?;

        self.duration = info.duration;
        self.current_path = Some(path.to_path_buf());
        self.position = Duration::ZERO;
        self.state = PlaybackState::Stopped;
        self.playing.store(false, Ordering::Relaxed);

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
        if self.current_path.is_none() {
            return Err(JalwaError::Playback("no file loaded".to_string()));
        }
        self.state = PlaybackState::Playing;
        self.playing.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            self.playing.store(false, Ordering::Relaxed);
        }
    }

    /// Stop playback and reset position
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.position = Duration::ZERO;
        self.playing.store(false, Ordering::Relaxed);
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
        self.position = position;
        Ok(())
    }

    /// Seek by a relative offset (can be negative)
    pub fn seek_relative(&mut self, offset_secs: f64) -> Result<()> {
        let new_pos = self.position.as_secs_f64() + offset_secs;
        let clamped = new_pos.max(0.0);
        let target = if let Some(dur) = self.duration {
            Duration::from_secs_f64(clamped.min(dur.as_secs_f64()))
        } else {
            Duration::from_secs_f64(clamped)
        };
        self.position = target;
        Ok(())
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Get current volume
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Toggle mute
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// Get mute state
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Get current playback state
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Get current position
    pub fn position(&self) -> Duration {
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
        PlaybackStatus {
            state: self.state,
            current_item: None, // Caller maps path -> UUID
            position: self.position,
            duration: self.duration,
            volume: self.volume,
            muted: self.muted,
        }
    }

    /// Get engine config
    pub fn config(&self) -> &EngineConfig {
        &self.config
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

    #[test]
    fn seek_relative() {
        let mut engine = PlaybackEngine::new(EngineConfig::default());
        engine.duration = Some(Duration::from_secs(300));
        engine.position = Duration::from_secs(100);

        engine.seek_relative(30.0).unwrap();
        assert_eq!(engine.position().as_secs(), 130);

        engine.seek_relative(-50.0).unwrap();
        assert_eq!(engine.position().as_secs(), 80);

        // Clamp to zero
        engine.seek_relative(-200.0).unwrap();
        assert_eq!(engine.position(), Duration::ZERO);

        // Clamp to duration
        engine.seek_relative(500.0).unwrap();
        assert_eq!(engine.position().as_secs(), 300);
    }
}
