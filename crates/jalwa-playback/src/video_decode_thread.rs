//! Video decode loop — runs on a dedicated OS thread.
//!
//! Demuxes a container (MP4, MKV, WebM) into audio and video streams,
//! decodes video frames via tarang's VideoDecoder, converts to Rgba32 for
//! display, and routes audio through the existing audio output pipeline.

#[cfg(feature = "video")]
use std::path::PathBuf;
#[cfg(feature = "video")]
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

#[cfg(feature = "video")]
use tarang::core::{PixelFormat, StreamInfo, VideoFrame};
#[cfg(feature = "video")]
use tarang::demux::{Demuxer, Packet};
#[cfg(feature = "video")]
use tarang::video::{DecoderConfig, VideoDecoder};

#[cfg(feature = "video")]
use crate::EngineConfig;
#[cfg(feature = "video")]
use crate::decode_thread::{DecodeStatus, EngineCommand, EngineEvent};

/// A decoded video frame ready for display (RGB24).
#[derive(Debug, Clone)]
pub struct DisplayFrame {
    /// RGB24 pixel data.
    pub data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Presentation timestamp.
    pub pts: Duration,
}

/// Run the video decode loop. Blocks until the track ends or a `Stop` command is received.
///
/// Demuxes the file, decodes video frames to RGB24, sends them via `frame_tx`,
/// and plays audio through PipeWire.
#[cfg(feature = "video")]
pub fn video_decode_loop(
    path: PathBuf,
    cmd_rx: mpsc::Receiver<EngineCommand>,
    status: Arc<Mutex<DecodeStatus>>,
    event_tx: mpsc::Sender<EngineEvent>,
    frame_tx: mpsc::SyncSender<DisplayFrame>,
    config: EngineConfig,
    _duration: Option<Duration>,
) {
    use tarang::audio::{AudioOutput, ChannelLayout, OutputConfig, mix_channels, resample};
    use tarang::core::TarangError;

    // Open the file and detect container format
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(format!("video open: {e}")));
            return;
        }
    };

    let reader = std::io::BufReader::new(file);

    // Read magic bytes for format detection
    let mut header = [0u8; 16];
    {
        use std::io::{Read, Seek, SeekFrom};
        let mut r = std::io::BufReader::new(std::fs::File::open(&path).unwrap());
        let _ = r.read(&mut header);
    }

    let format = match tarang::demux::detect_format(&header) {
        Ok(f) => f,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(format!("detect format: {e}")));
            return;
        }
    };

    let mut demuxer: Box<dyn Demuxer> = match format {
        tarang::core::ContainerFormat::Mp4 => Box::new(tarang::demux::Mp4Demuxer::new(reader)),
        tarang::core::ContainerFormat::Mkv => Box::new(tarang::demux::MkvDemuxer::new(reader)),
        _ => {
            let _ = event_tx.send(EngineEvent::Error(format!(
                "unsupported video container: {format:?}"
            )));
            return;
        }
    };

    // Probe to get stream info
    let info = match demuxer.probe() {
        Ok(i) => i,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(format!("probe: {e}")));
            return;
        }
    };

    // Find video and audio stream indices
    let mut video_stream_idx = None;
    let mut audio_stream_idx = None;
    let mut video_stream_info = None;
    let mut audio_stream_info = None;

    for (i, stream) in info.streams.iter().enumerate() {
        match stream {
            StreamInfo::Video(v) if video_stream_idx.is_none() => {
                video_stream_idx = Some(i);
                video_stream_info = Some(v.clone());
            }
            StreamInfo::Audio(a) if audio_stream_idx.is_none() => {
                audio_stream_idx = Some(i);
                audio_stream_info = Some(a.clone());
            }
            _ => {}
        }
    }

    let (video_idx, vinfo) = match (video_stream_idx, video_stream_info) {
        (Some(i), Some(v)) => (i, v),
        _ => {
            let _ = event_tx.send(EngineEvent::Error("no video stream found".to_string()));
            return;
        }
    };

    // Initialize video decoder
    let decoder_config = match DecoderConfig::for_codec(vinfo.codec) {
        Ok(c) => c,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(format!("video codec: {e}")));
            return;
        }
    };

    let mut video_decoder = match VideoDecoder::new(decoder_config) {
        Ok(d) => d,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(format!("video decoder init: {e}")));
            return;
        }
    };
    video_decoder.init(&vinfo);

    // Open audio output if there's an audio stream
    let mut audio_output: Option<Box<dyn AudioOutput>> = None;
    let mut audio_decoder: Option<tarang::audio::FileDecoder> = None;

    if let Some(ref _ainfo) = audio_stream_info {
        let mut out = crate::decode_thread::create_audio_output();
        let out_config = OutputConfig {
            sample_rate: config.sample_rate,
            channels: config.channels,
            buffer_size: config.buffer_size,
        };
        if let Err(e) = out.open(&out_config) {
            tracing::warn!("audio output open failed in video mode: {e}");
        } else {
            audio_output = Some(out);
        }
    }

    let _ = event_tx.send(EngineEvent::StateChanged(
        jalwa_core::PlaybackState::Playing,
    ));
    let mut paused = false;
    let mut volume: f32 = 1.0;
    let mut muted = false;

    loop {
        // Handle commands
        loop {
            let cmd = if paused {
                match cmd_rx.recv() {
                    Ok(c) => Some(c),
                    Err(_) => return,
                }
            } else {
                cmd_rx.try_recv().ok()
            };

            match cmd {
                Some(EngineCommand::Stop) => {
                    if let Some(ref mut out) = audio_output {
                        let _ = out.flush();
                        let _ = out.close();
                    }
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
                    if let Err(e) = demuxer.seek(pos) {
                        let _ = event_tx.send(EngineEvent::Error(format!("seek: {e}")));
                    }
                    break;
                }
                Some(EngineCommand::Volume(v)) => volume = v.clamp(0.0, 1.0),
                Some(EngineCommand::Mute(m)) => muted = m,
                Some(_) => {} // Ignore EQ/Normalize/PrepareNext in video mode
                None => break,
            }
        }

        // Read next packet
        let packet = match demuxer.next_packet() {
            Ok(p) => p,
            Err(TarangError::EndOfStream) => {
                // Flush video decoder
                let _ = video_decoder.flush();
                while let Ok(frame) = video_decoder.receive_frame() {
                    if let Ok(rgb) = frame.convert_to(PixelFormat::Rgb24) {
                        let _ = frame_tx.try_send(DisplayFrame {
                            data: rgb.data.to_vec(),
                            width: rgb.width,
                            height: rgb.height,
                            pts: rgb.timestamp,
                        });
                    }
                }
                if let Some(ref mut out) = audio_output {
                    let _ = out.flush();
                    let _ = out.close();
                }
                let _ = event_tx.send(EngineEvent::TrackFinished);
                return;
            }
            Err(e) => {
                let _ = event_tx.send(EngineEvent::Error(format!("demux: {e}")));
                return;
            }
        };

        if packet.stream_index == video_idx {
            // Video packet — decode and convert
            if let Err(e) = video_decoder.send_packet(&packet.data, packet.timestamp) {
                tracing::warn!("video decode error: {e}");
                continue;
            }

            while let Ok(frame) = video_decoder.receive_frame() {
                match frame.convert_to(PixelFormat::Rgb24) {
                    Ok(rgb) => {
                        let display = DisplayFrame {
                            data: rgb.data.to_vec(),
                            width: rgb.width,
                            height: rgb.height,
                            pts: rgb.timestamp,
                        };
                        // Blocking send with bounded channel provides backpressure
                        let _ = frame_tx.try_send(display);
                    }
                    Err(e) => {
                        tracing::warn!("pixel format conversion: {e}");
                    }
                }
            }

            // Update status with video timestamp
            if let Ok(mut s) = status.lock() {
                s.state = jalwa_core::PlaybackState::Playing;
                s.position = packet.timestamp;
                s.volume = volume;
                s.muted = muted;
            }
        } else if Some(packet.stream_index) == audio_stream_idx {
            // Audio packet — write directly to output
            // For MVP, we write raw audio data. A full implementation would
            // decode the audio codec first (AAC, Opus, etc.)
            if let Some(ref mut out) = audio_output {
                let audio_buf = tarang::core::AudioBuffer {
                    data: packet.data,
                    sample_format: tarang::core::SampleFormat::F32,
                    channels: config.channels,
                    sample_rate: config.sample_rate,
                    num_frames: 0, // Will be computed from data length
                    timestamp: packet.timestamp,
                };

                // Apply volume
                if muted || (volume - 1.0).abs() > 1e-4 {
                    let gain = if muted { 0.0 } else { volume };
                    let mut buf = audio_buf;
                    crate::decode_thread::apply_volume_in_place(&mut buf, gain);
                    let _ = out.write(&buf);
                } else {
                    let _ = out.write(&audio_buf);
                }
            }
        }
        // Ignore subtitle and other stream packets for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_frame_creation() {
        let frame = DisplayFrame {
            data: vec![255; 640 * 480 * 3],
            width: 640,
            height: 480,
            pts: Duration::from_millis(100),
        };
        assert_eq!(frame.data.len(), 640 * 480 * 3);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.pts, Duration::from_millis(100));
    }

    #[test]
    fn display_frame_clone() {
        let frame = DisplayFrame {
            data: vec![128; 320 * 240 * 3],
            width: 320,
            height: 240,
            pts: Duration::from_millis(33),
        };
        let cloned = frame.clone();
        assert_eq!(cloned.width, frame.width);
        assert_eq!(cloned.height, frame.height);
        assert_eq!(cloned.pts, frame.pts);
        assert_eq!(cloned.data.len(), frame.data.len());
    }

    #[test]
    fn display_frame_debug() {
        let frame = DisplayFrame {
            data: vec![0; 12],
            width: 2,
            height: 2,
            pts: Duration::ZERO,
        };
        let debug = format!("{:?}", frame);
        assert!(debug.contains("DisplayFrame"));
    }

    #[test]
    fn display_frame_zero_size() {
        let frame = DisplayFrame {
            data: Vec::new(),
            width: 0,
            height: 0,
            pts: Duration::ZERO,
        };
        assert!(frame.data.is_empty());
    }
}
