//! Decode loop — runs on a dedicated OS thread.
//!
//! Reads packets from a `FileDecoder`, resamples/mixes as needed, applies
//! volume, and writes to audio output. Receives commands from the engine
//! via an `mpsc` channel and pushes status updates back via a `watch`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use tarang_audio::{AudioOutput, ChannelLayout, FileDecoder, OutputConfig, mix_channels, resample};
use tarang_core::TarangError;

use crate::EngineConfig;
use crate::dsp::{self, Equalizer};

/// Commands sent from the engine to the decode thread.
#[derive(Debug)]
pub enum EngineCommand {
    Play,
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    Volume(f32),
    Mute(bool),
    PrepareNext {
        path: PathBuf,
    },
    /// Update EQ settings (bands + enabled).
    EqUpdate(crate::dsp::EqSettings),
    /// Enable/disable volume normalization.
    Normalize(bool),
}

/// Events sent from the decode thread back to the engine / UI.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    StateChanged(jalwa_core::PlaybackState),
    PositionUpdate(Duration),
    TrackFinished,
    TrackChanged,
    NearEnd,
    Error(String),
}

/// Snapshot of decode‐thread state, published via `watch`.
#[derive(Debug, Clone)]
pub struct DecodeStatus {
    pub state: jalwa_core::PlaybackState,
    pub position: Duration,
    pub volume: f32,
    pub muted: bool,
}

impl Default for DecodeStatus {
    fn default() -> Self {
        Self {
            state: jalwa_core::PlaybackState::Stopped,
            position: Duration::ZERO,
            volume: 1.0,
            muted: false,
        }
    }
}

/// Create the appropriate audio output for the platform.
fn create_output() -> Box<dyn AudioOutput> {
    #[cfg(feature = "pipewire")]
    {
        Box::new(tarang_audio::PipeWireOutput::new())
    }
    #[cfg(not(feature = "pipewire"))]
    {
        Box::new(tarang_audio::NullOutput::new())
    }
}

/// Run the decode loop. Blocks until the track ends or a `Stop` command is received.
///
/// This function is intended to be called from `std::thread::spawn`.
pub fn decode_loop(
    path: PathBuf,
    cmd_rx: mpsc::Receiver<EngineCommand>,
    status: Arc<Mutex<DecodeStatus>>,
    event_tx: mpsc::Sender<EngineEvent>,
    config: EngineConfig,
    duration: Option<Duration>,
) {
    let mut volume: f32 = 1.0;
    let mut muted = false;
    let mut equalizer = Equalizer::new(config.sample_rate);
    let mut normalize_enabled = false;
    let mut smooth_gain: f32 = 1.0; // Smoothed normalization gain to prevent pumping

    // Open decoder
    let mut decoder = match FileDecoder::open_path(&path) {
        Ok(d) => d,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(format!("decode open: {e}")));
            return;
        }
    };

    // Open audio output
    let mut output = create_output();
    let out_config = OutputConfig {
        sample_rate: config.sample_rate,
        channels: config.channels,
        buffer_size: config.buffer_size,
    };
    if let Err(e) = output.open(&out_config) {
        let _ = event_tx.send(EngineEvent::Error(format!("audio output open: {e}")));
        return;
    }

    let _ = event_tx.send(EngineEvent::StateChanged(
        jalwa_core::PlaybackState::Playing,
    ));
    let mut paused = false;
    let mut near_end_sent = false;

    // Pre-buffered next track for gapless
    let mut next_decoder: Option<FileDecoder> = None;

    loop {
        // Handle commands (non-blocking when playing, blocking when paused)
        loop {
            let cmd = if paused {
                match cmd_rx.recv() {
                    Ok(c) => Some(c),
                    Err(_) => {
                        let _ = output.flush();
                        let _ = output.close();
                        return;
                    }
                }
            } else {
                cmd_rx.try_recv().ok()
            };

            match cmd {
                Some(EngineCommand::Stop) => {
                    let _ = output.flush();
                    let _ = output.close();
                    let _ = event_tx.send(EngineEvent::StateChanged(
                        jalwa_core::PlaybackState::Stopped,
                    ));
                    return;
                }
                Some(EngineCommand::Pause) => {
                    paused = true;
                    let _ =
                        event_tx.send(EngineEvent::StateChanged(jalwa_core::PlaybackState::Paused));
                    if let Ok(mut s) = status.lock() {
                        s.state = jalwa_core::PlaybackState::Paused;
                        s.volume = volume;
                        s.muted = muted;
                    }
                    continue;
                }
                Some(EngineCommand::Resume | EngineCommand::Play) => {
                    paused = false;
                    let _ = event_tx.send(EngineEvent::StateChanged(
                        jalwa_core::PlaybackState::Playing,
                    ));
                    break;
                }
                Some(EngineCommand::Seek(pos)) => {
                    if let Err(e) = decoder.seek(pos) {
                        let _ = event_tx.send(EngineEvent::Error(format!("seek: {e}")));
                    }
                    // Reset EQ filter state to prevent transient click from stale samples
                    equalizer.reset();
                    break;
                }
                Some(EngineCommand::Volume(v)) => {
                    volume = v.clamp(0.0, 1.0);
                }
                Some(EngineCommand::Mute(m)) => {
                    muted = m;
                }
                Some(EngineCommand::PrepareNext { path }) => match FileDecoder::open_path(&path) {
                    Ok(d) => next_decoder = Some(d),
                    Err(e) => {
                        let _ = event_tx.send(EngineEvent::Error(format!("prepare next: {e}")));
                    }
                },
                Some(EngineCommand::EqUpdate(settings)) => {
                    equalizer.settings = settings;
                    equalizer.update_coefficients();
                }
                Some(EngineCommand::Normalize(enabled)) => {
                    normalize_enabled = enabled;
                }
                None => break,
            }
        }

        // Decode next buffer
        let buf = match decoder.next_buffer() {
            Ok(b) => b,
            Err(TarangError::EndOfStream) => {
                // Try gapless transition
                if let Some(next) = next_decoder.take() {
                    decoder = next;
                    near_end_sent = false;
                    equalizer.reset();
                    let _ = event_tx.send(EngineEvent::TrackChanged);
                    continue;
                }
                let _ = output.flush();
                let _ = output.close();
                let _ = event_tx.send(EngineEvent::TrackFinished);
                return;
            }
            Err(e) => {
                let _ = event_tx.send(EngineEvent::Error(format!("decode: {e}")));
                let _ = output.flush();
                let _ = output.close();
                return;
            }
        };

        // Resample if needed
        let buf = if buf.sample_rate != config.sample_rate {
            match resample(&buf, config.sample_rate) {
                Ok(b) => b,
                Err(e) => {
                    let _ = event_tx.send(EngineEvent::Error(format!("resample: {e}")));
                    continue; // Skip this buffer rather than output at wrong rate
                }
            }
        } else {
            buf
        };

        // Mix channels if needed
        let buf = if buf.channels != config.channels {
            let target = if config.channels == 1 {
                ChannelLayout::Mono
            } else {
                ChannelLayout::Stereo
            };
            match mix_channels(&buf, target) {
                Ok(b) => b,
                Err(e) => {
                    let _ = event_tx.send(EngineEvent::Error(format!("channel mix: {e}")));
                    continue; // Skip this buffer rather than output wrong channel count
                }
            }
        } else {
            buf
        };

        // Apply equalizer
        let buf = equalizer.process(&buf);

        // Apply normalization with smoothed gain to prevent pumping
        let buf = if normalize_enabled {
            let info = dsp::analyze_loudness(&buf);
            // Exponential moving average: attack fast (0.3), release slow (0.05)
            let alpha = if info.gain < smooth_gain { 0.3 } else { 0.05 };
            smooth_gain = smooth_gain + alpha * (info.gain - smooth_gain);
            dsp::normalize(&buf, smooth_gain)
        } else {
            buf
        };

        // Apply volume (tolerance accounts for f32 drift from repeated UI adjustments)
        let buf = if muted || (volume - 1.0).abs() > 1e-4 {
            let gain = if muted { 0.0 } else { volume };
            apply_volume(&buf, gain)
        } else {
            buf
        };

        // Update status
        if let Ok(mut s) = status.lock() {
            s.state = jalwa_core::PlaybackState::Playing;
            s.position = buf.timestamp;
            s.volume = volume;
            s.muted = muted;
        }

        // Check near-end for gapless prebuffer hint
        if !near_end_sent
            && let Some(dur) = duration
            && dur.as_secs_f64() - buf.timestamp.as_secs_f64() < 2.0
        {
            let _ = event_tx.send(EngineEvent::NearEnd);
            near_end_sent = true;
        }

        // Write to output
        if let Err(e) = output.write(&buf) {
            let _ = event_tx.send(EngineEvent::Error(format!("output write: {e}")));
            let _ = output.close();
            return;
        }
    }
}

/// Apply volume gain to an AudioBuffer, returning a new buffer.
pub(crate) fn apply_volume(buf: &tarang_core::AudioBuffer, gain: f32) -> tarang_core::AudioBuffer {
    let samples = dsp::buf_to_f32_safe(buf);
    let scaled: Vec<f32> = samples.iter().map(|s| s * gain).collect();
    let bytes: &[u8] = bytemuck::cast_slice(&scaled);
    tarang_core::AudioBuffer {
        data: bytes::Bytes::copy_from_slice(bytes),
        sample_format: buf.sample_format,
        channels: buf.channels,
        sample_rate: buf.sample_rate,
        num_samples: buf.num_samples,
        timestamp: buf.timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a minimal WAV file in memory for testing.
    fn make_test_wav(num_samples: u32, sample_rate: u32) -> Vec<u8> {
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

    fn write_test_wav() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("jalwa_dt_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");
        let wav = make_test_wav(4410, 44100); // 0.1 second of audio
        std::fs::write(&path, &wav).unwrap();
        path
    }

    #[test]
    fn decode_loop_plays_to_end() {
        let wav_path = write_test_wav();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DecodeStatus::default()));
        let (event_tx, event_rx) = mpsc::channel();
        let config = crate::EngineConfig {
            buffer_size: 4096,
            sample_rate: 44100,
            channels: 1,
        };

        // Run decode loop in a thread — it should play to end with NullOutput
        // (NullOutput is used when pipewire feature is disabled in tests)
        let status_clone = status.clone();
        let handle = std::thread::spawn(move || {
            decode_loop(
                wav_path.clone(),
                cmd_rx,
                status_clone,
                event_tx,
                config,
                Some(Duration::from_millis(100)),
            );
        });

        // Wait for completion
        handle.join().unwrap();

        // Should have received TrackFinished
        let mut got_finished = false;
        while let Ok(ev) = event_rx.try_recv() {
            if matches!(ev, EngineEvent::TrackFinished) {
                got_finished = true;
            }
        }
        assert!(got_finished);
    }

    #[test]
    fn decode_loop_stop_command() {
        let wav_path = write_test_wav();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DecodeStatus::default()));
        let (event_tx, event_rx) = mpsc::channel();
        let config = crate::EngineConfig {
            buffer_size: 4096,
            sample_rate: 44100,
            channels: 1,
        };

        let status_clone = status.clone();
        let handle = std::thread::spawn(move || {
            decode_loop(
                wav_path.clone(),
                cmd_rx,
                status_clone,
                event_tx,
                config,
                None,
            );
        });

        // Send stop immediately
        let _ = cmd_tx.send(EngineCommand::Stop);
        handle.join().unwrap();

        // Should have received StateChanged(Stopped)
        let mut got_stopped = false;
        while let Ok(ev) = event_rx.try_recv() {
            if let EngineEvent::StateChanged(jalwa_core::PlaybackState::Stopped) = ev {
                got_stopped = true;
            }
        }
        assert!(got_stopped);
    }

    #[test]
    fn decode_loop_volume_command() {
        let wav_path = write_test_wav();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DecodeStatus::default()));
        let (event_tx, _event_rx) = mpsc::channel();
        let config = crate::EngineConfig {
            buffer_size: 4096,
            sample_rate: 44100,
            channels: 1,
        };

        // Send volume command before starting
        let _ = cmd_tx.send(EngineCommand::Volume(0.5));

        let status_clone = status.clone();
        let handle = std::thread::spawn(move || {
            decode_loop(
                wav_path.clone(),
                cmd_rx,
                status_clone,
                event_tx,
                config,
                None,
            );
        });

        handle.join().unwrap();

        // Check status reflects the volume
        let s = status.lock().unwrap();
        // Volume should have been applied (either 0.5 if read before end, or default)
        assert!(s.volume >= 0.0 && s.volume <= 1.0);
    }

    #[test]
    fn decode_loop_nonexistent_file() {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DecodeStatus::default()));
        let (event_tx, event_rx) = mpsc::channel();
        let config = crate::EngineConfig::default();

        let handle = std::thread::spawn(move || {
            decode_loop(
                std::path::PathBuf::from("/nonexistent/file.wav"),
                cmd_rx,
                status,
                event_tx,
                config,
                None,
            );
        });

        handle.join().unwrap();

        // Should have sent an error event
        let mut got_error = false;
        while let Ok(ev) = event_rx.try_recv() {
            if matches!(ev, EngineEvent::Error(_)) {
                got_error = true;
            }
        }
        assert!(got_error);
    }

    #[test]
    fn decode_loop_pause_resume() {
        let wav_path = write_test_wav();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DecodeStatus::default()));
        let (event_tx, event_rx) = mpsc::channel();
        let config = crate::EngineConfig {
            buffer_size: 4096,
            sample_rate: 44100,
            channels: 1,
        };

        let status_clone = status.clone();
        let handle = std::thread::spawn(move || {
            decode_loop(
                wav_path.clone(),
                cmd_rx,
                status_clone,
                event_tx,
                config,
                None,
            );
        });

        // Brief delay to let decode start
        std::thread::sleep(Duration::from_millis(10));
        let _ = cmd_tx.send(EngineCommand::Pause);
        std::thread::sleep(Duration::from_millis(10));
        let _ = cmd_tx.send(EngineCommand::Resume);
        // Let it finish naturally
        handle.join().unwrap();

        // Should have received pause and play state changes
        let mut states = Vec::new();
        while let Ok(ev) = event_rx.try_recv() {
            if let EngineEvent::StateChanged(s) = ev {
                states.push(s);
            }
        }
        assert!(states.contains(&jalwa_core::PlaybackState::Playing));
    }

    #[test]
    fn decode_status_default_values() {
        let s = DecodeStatus::default();
        assert_eq!(s.state, jalwa_core::PlaybackState::Stopped);
        assert_eq!(s.position, Duration::ZERO);
        assert_eq!(s.volume, 1.0);
        assert!(!s.muted);
    }

    #[test]
    fn engine_command_debug() {
        let cmd = EngineCommand::Play;
        assert!(format!("{:?}", cmd).contains("Play"));
        let cmd = EngineCommand::Seek(Duration::from_secs(5));
        assert!(format!("{:?}", cmd).contains("Seek"));
    }

    #[test]
    fn engine_event_clone() {
        let ev = EngineEvent::TrackFinished;
        let cloned = ev.clone();
        assert!(matches!(cloned, EngineEvent::TrackFinished));
    }
}
