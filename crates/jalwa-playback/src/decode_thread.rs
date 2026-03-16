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
    PrepareNext { path: PathBuf },
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

    let _ = event_tx.send(EngineEvent::StateChanged(jalwa_core::PlaybackState::Playing));
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
                    let _ = event_tx.send(EngineEvent::StateChanged(jalwa_core::PlaybackState::Stopped));
                    return;
                }
                Some(EngineCommand::Pause) => {
                    paused = true;
                    let _ = event_tx.send(EngineEvent::StateChanged(jalwa_core::PlaybackState::Paused));
                    if let Ok(mut s) = status.lock() {
                        s.state = jalwa_core::PlaybackState::Paused;
                        s.volume = volume;
                        s.muted = muted;
                    }
                    continue;
                }
                Some(EngineCommand::Resume | EngineCommand::Play) => {
                    paused = false;
                    let _ = event_tx.send(EngineEvent::StateChanged(jalwa_core::PlaybackState::Playing));
                    break;
                }
                Some(EngineCommand::Seek(pos)) => {
                    if let Err(e) = decoder.seek(pos) {
                        let _ = event_tx.send(EngineEvent::Error(format!("seek: {e}")));
                    }
                    break;
                }
                Some(EngineCommand::Volume(v)) => {
                    volume = v.clamp(0.0, 1.0);
                }
                Some(EngineCommand::Mute(m)) => {
                    muted = m;
                }
                Some(EngineCommand::PrepareNext { path }) => {
                    match FileDecoder::open_path(&path) {
                        Ok(d) => next_decoder = Some(d),
                        Err(e) => {
                            let _ = event_tx.send(EngineEvent::Error(format!("prepare next: {e}")));
                        }
                    }
                }
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
                    tracing::warn!("resample error: {e}");
                    buf
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
                    tracing::warn!("channel mix error: {e}");
                    buf
                }
            }
        } else {
            buf
        };

        // Apply equalizer
        let buf = equalizer.process(&buf);

        // Apply normalization
        let buf = if normalize_enabled {
            let info = dsp::analyze_loudness(&buf);
            dsp::normalize(&buf, info.gain)
        } else {
            buf
        };

        // Apply volume
        let buf = if muted || (volume - 1.0).abs() > f32::EPSILON {
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
        if !near_end_sent {
            if let Some(dur) = duration {
                if dur.as_secs_f64() - buf.timestamp.as_secs_f64() < 2.0 {
                    let _ = event_tx.send(EngineEvent::NearEnd);
                    near_end_sent = true;
                }
            }
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
    let samples: &[f32] = unsafe {
        std::slice::from_raw_parts(buf.data.as_ptr() as *const f32, buf.data.len() / 4)
    };
    let scaled: Vec<f32> = samples.iter().map(|s| s * gain).collect();
    let bytes = unsafe {
        std::slice::from_raw_parts(scaled.as_ptr() as *const u8, scaled.len() * 4)
    };
    tarang_core::AudioBuffer {
        data: bytes::Bytes::copy_from_slice(bytes),
        sample_format: buf.sample_format,
        channels: buf.channels,
        sample_rate: buf.sample_rate,
        num_samples: buf.num_samples,
        timestamp: buf.timestamp,
    }
}
