//! Audio DSP — equalizer and volume normalization.
//!
//! Backed by [dhvani](https://crates.io/crates/dhvani) for biquad EQ,
//! gain smoothing, and loudness analysis. Bridge functions convert between
//! tarang's `AudioBuffer` (Bytes-based) and dhvani's f32 processing.

#[cfg(feature = "tarang")]
use tarang::core::AudioBuffer;

// Re-export dhvani types under jalwa's public API.
pub use dhvani::dsp::graphic_eq::GraphicEqSettings as EqSettings;
pub use dhvani::dsp::graphic_eq::ISO_BANDS as EQ_BANDS;
pub use dhvani::dsp::{GainSmoother, GainSmootherParams};

// ---- Volume Normalization ----

/// Loudness analysis result for a buffer or track.
#[derive(Debug, Clone, Copy)]
pub struct LoudnessInfo {
    /// RMS level in linear scale (0.0..1.0+)
    pub rms: f32,
    /// Peak sample value (absolute)
    pub peak: f32,
    /// Gain to apply for normalization to target RMS (linear)
    pub gain: f32,
}

/// Target RMS for normalization (~-18 dBFS, typical ReplayGain reference).
pub const TARGET_RMS: f32 = 0.125;

/// Analyze a buffer's loudness and compute normalization gain.
#[cfg(feature = "tarang")]
pub fn analyze_loudness(buf: &AudioBuffer) -> LoudnessInfo {
    let samples = buf_to_f32(buf);
    if samples.is_empty() {
        return LoudnessInfo {
            rms: 0.0,
            peak: 0.0,
            gain: 1.0,
        };
    }

    // Create a temporary dhvani buffer for analysis
    let dbuf = to_dhvani(&samples, buf.channels, buf.sample_rate);
    let rms = dbuf.rms();
    let peak = dbuf.peak();
    let gain = dhvani::analysis::suggest_gain(&dbuf, TARGET_RMS);

    LoudnessInfo { rms, peak, gain }
}

/// Apply normalization gain to a buffer, with peak limiting to prevent clipping.
#[cfg(feature = "tarang")]
pub fn normalize(buf: &AudioBuffer, gain: f32) -> AudioBuffer {
    let samples = buf_to_f32(buf);
    let limited_gain = if gain > 1.0 {
        // Find peak after gain to prevent clipping
        let max_after = samples
            .iter()
            .map(|s| (s * gain).abs())
            .fold(0.0f32, f32::max);
        if max_after > 1.0 {
            gain / max_after
        } else {
            gain
        }
    } else {
        gain
    };

    let scaled: Vec<f32> = samples.iter().map(|s| s * limited_gain).collect();
    f32_to_buf(&scaled, buf)
}

// ---- 10-Band Graphic Equalizer (dhvani-backed) ----

/// 10-band graphic equalizer processor.
///
/// Wraps dhvani's `GraphicEq` with tarang `AudioBuffer` integration.
/// Call `process()` on each decoded buffer in the decode loop.
pub struct Equalizer {
    inner: dhvani::dsp::GraphicEq,
    pub settings: EqSettings,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            inner: dhvani::dsp::GraphicEq::new(sample_rate, 2),
            settings: EqSettings::default(),
        }
    }

    /// Recompute filter coefficients from current settings.
    pub fn update_coefficients(&mut self) {
        self.inner.set_settings(self.settings.clone());
    }

    /// Reset filter state (call on seek or track change).
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Process an audio buffer through the 10-band EQ.
    #[cfg(feature = "tarang")]
    pub fn process(&mut self, buf: &AudioBuffer) -> AudioBuffer {
        if !self.settings.enabled || self.settings.is_flat() {
            return buf.clone();
        }

        let samples = buf_to_f32(buf);

        // Convert to dhvani buffer, process, convert back
        let mut dbuf = to_dhvani(&samples, buf.channels, buf.sample_rate);
        self.inner.set_enabled(true);
        self.inner.process(&mut dbuf);
        f32_to_buf(&dbuf.samples, buf)
    }
}

// ---- Helpers ----

/// Safely interpret an AudioBuffer's bytes as `&[f32]`.
///
/// If the underlying `Bytes` is not 4-byte aligned we copy into an
/// aligned temporary. The returned `Cow` avoids the copy when alignment
/// is already correct (the common path).
///
/// # Alignment requirement
///
/// Each f32 sample occupies 4 bytes, so `buf.data.len()` must be a
/// multiple of 4. If a corrupted or partial buffer has trailing bytes
/// that don't form a complete f32, those bytes are silently truncated
/// (equivalent to `chunks_exact` ignoring the remainder).
#[cfg(feature = "tarang")]
pub(crate) fn buf_to_f32_safe(buf: &AudioBuffer) -> std::borrow::Cow<'_, [f32]> {
    if buf.data.is_empty() {
        return std::borrow::Cow::Borrowed(&[]);
    }

    // Guard: if the buffer length is not 4-byte aligned, log a warning.
    // Trailing bytes that don't form a complete f32 sample are truncated.
    if !buf.data.len().is_multiple_of(4) {
        tracing::warn!(
            "AudioBuffer has {} bytes which is not 4-byte aligned; \
             truncating {} trailing byte(s) for f32 conversion",
            buf.data.len(),
            buf.data.len() % 4,
        );
    }

    match bytemuck::try_cast_slice::<u8, f32>(&buf.data) {
        Ok(s) => std::borrow::Cow::Borrowed(s),
        Err(_) => {
            // Fallback: copy into an aligned Vec, truncating any trailing bytes
            let n = buf.data.len() / 4;
            let mut out = vec![0.0f32; n];
            for (i, chunk) in buf.data.chunks_exact(4).enumerate() {
                out[i] = f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
            std::borrow::Cow::Owned(out)
        }
    }
}

/// Legacy alias used by existing call sites that only need a `&[f32]` view.
#[cfg(feature = "tarang")]
fn buf_to_f32(buf: &AudioBuffer) -> std::borrow::Cow<'_, [f32]> {
    buf_to_f32_safe(buf)
}

/// Construct a tarang AudioBuffer from f32 samples, copying the metadata from a template.
#[cfg(feature = "tarang")]
fn f32_to_buf(samples: &[f32], template: &AudioBuffer) -> AudioBuffer {
    let bytes: &[u8] = bytemuck::cast_slice(samples);
    AudioBuffer {
        data: bytes::Bytes::copy_from_slice(bytes),
        sample_format: template.sample_format,
        channels: template.channels,
        sample_rate: template.sample_rate,
        num_frames: template.num_frames,
        timestamp: template.timestamp,
    }
}

/// Convert f32 samples + metadata into a dhvani AudioBuffer for processing.
#[cfg(feature = "tarang")]
fn to_dhvani(samples: &[f32], channels: u16, sample_rate: u32) -> dhvani::buffer::AudioBuffer {
    dhvani::buffer::AudioBuffer::from_interleaved(samples.to_vec(), channels as u32, sample_rate)
        .unwrap_or_else(|_| dhvani::buffer::AudioBuffer::silence(channels as u32, 0, sample_rate))
}

#[cfg(all(test, feature = "tarang"))]
mod tarang_tests {
    use super::*;
    use bytes::Bytes;
    use std::time::Duration;
    use tarang::core::SampleFormat;

    fn make_buf(samples: &[f32], channels: u16, sample_rate: u32) -> AudioBuffer {
        let bytes: &[u8] = bytemuck::cast_slice(samples);
        AudioBuffer {
            data: Bytes::copy_from_slice(bytes),
            sample_format: SampleFormat::F32,
            channels,
            sample_rate,
            num_frames: samples.len() / channels as usize,
            timestamp: Duration::ZERO,
        }
    }

    fn make_sine(freq: f64, sr: u32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f64 / sr as f64 * freq * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect()
    }

    // ---- Normalization tests ----

    #[test]
    fn loudness_silent_buffer() {
        let buf = make_buf(&[0.0; 1000], 1, 44100);
        let info = analyze_loudness(&buf);
        assert_eq!(info.rms, 0.0);
        assert_eq!(info.peak, 0.0);
        assert_eq!(info.gain, 1.0);
    }

    #[test]
    fn loudness_full_scale_sine() {
        let samples = make_sine(440.0, 44100, 44100);
        let buf = make_buf(&samples, 1, 44100);
        let info = analyze_loudness(&buf);
        assert!(info.rms > 0.5 && info.rms < 0.8, "rms={}", info.rms);
        assert!(info.peak > 0.99, "peak={}", info.peak);
        assert!(info.gain < 1.0, "gain={}", info.gain);
    }

    #[test]
    fn loudness_quiet_signal() {
        let samples: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin() * 0.01).collect();
        let buf = make_buf(&samples, 1, 44100);
        let info = analyze_loudness(&buf);
        assert!(info.gain > 1.0, "quiet signal should get positive gain");
        assert!(info.gain <= 10.0, "gain should be clamped");
    }

    #[test]
    fn normalize_prevents_clipping() {
        let samples: Vec<f32> = vec![0.5; 100];
        let buf = make_buf(&samples, 1, 44100);
        let result = normalize(&buf, 3.0);
        let out = buf_to_f32(&result);
        for &s in out.iter() {
            assert!(s.abs() <= 1.0, "sample {} exceeds 1.0", s);
        }
    }

    #[test]
    fn normalize_attenuate() {
        let samples: Vec<f32> = vec![0.8; 100];
        let buf = make_buf(&samples, 1, 44100);
        let result = normalize(&buf, 0.5);
        let out = buf_to_f32(&result);
        for &s in out.iter() {
            assert!((s - 0.4).abs() < 0.001, "expected 0.4, got {}", s);
        }
    }

    #[test]
    fn loudness_empty_buffer() {
        let buf = make_buf(&[], 1, 44100);
        let info = analyze_loudness(&buf);
        assert_eq!(info.gain, 1.0);
    }

    // ---- Equalizer tests ----

    #[test]
    fn eq_settings_default_is_flat() {
        let eq = EqSettings::default();
        assert!(!eq.enabled);
        assert!(eq.is_flat());
    }

    #[test]
    fn eq_settings_set_band() {
        let mut eq = EqSettings {
            enabled: true,
            ..Default::default()
        };
        eq.set_band(0, 6.0);
        assert_eq!(eq.bands[0], 6.0);
        assert!(!eq.is_flat());
    }

    #[test]
    fn eq_settings_clamp() {
        let mut eq = EqSettings::default();
        eq.set_band(0, 20.0);
        assert_eq!(eq.bands[0], 12.0);
        eq.set_band(0, -20.0);
        assert_eq!(eq.bands[0], -12.0);
    }

    #[test]
    fn eq_settings_out_of_range_band() {
        let mut eq = EqSettings::default();
        eq.set_band(99, 6.0);
        assert!(eq.is_flat());
    }

    #[test]
    fn eq_band_names() {
        assert_eq!(EqSettings::band_name(0), "31 Hz");
        assert_eq!(EqSettings::band_name(5), "1 kHz");
        assert_eq!(EqSettings::band_name(9), "16 kHz");
        assert_eq!(EqSettings::band_name(10), "?");
    }

    #[test]
    fn eq_flat_passthrough() {
        let samples = make_sine(1000.0, 48000, 4800);
        let buf = make_buf(&samples, 1, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.enabled = true;
        let out = eq.process(&buf);
        assert_eq!(out.data, buf.data);
    }

    #[test]
    fn eq_disabled_passthrough() {
        let samples = make_sine(1000.0, 48000, 4800);
        let buf = make_buf(&samples, 1, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.set_band(5, 12.0);
        let out = eq.process(&buf);
        assert_eq!(out.data, buf.data);
    }

    #[test]
    fn eq_boost_changes_signal() {
        let samples = make_sine(1000.0, 48000, 4800);
        let buf = make_buf(&samples, 1, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.enabled = true;
        eq.settings.set_band(5, 12.0);
        eq.update_coefficients();
        let out = eq.process(&buf);
        assert_ne!(out.data, buf.data);
        let in_rms = rms(&buf_to_f32(&buf));
        let out_rms = rms(&buf_to_f32(&out));
        assert!(
            out_rms > in_rms,
            "boost should increase RMS: in={in_rms}, out={out_rms}"
        );
    }

    #[test]
    fn eq_cut_changes_signal() {
        let samples = make_sine(1000.0, 48000, 4800);
        let buf = make_buf(&samples, 1, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.enabled = true;
        eq.settings.set_band(5, -12.0);
        eq.update_coefficients();
        let out = eq.process(&buf);
        let in_rms = rms(&buf_to_f32(&buf));
        let out_rms = rms(&buf_to_f32(&out));
        assert!(
            out_rms < in_rms,
            "cut should decrease RMS: in={in_rms}, out={out_rms}"
        );
    }

    #[test]
    fn eq_reset_clears_state() {
        let mut eq = Equalizer::new(48000);
        // Reset should not panic
        eq.reset();
    }

    #[test]
    fn eq_stereo() {
        let mut samples = Vec::new();
        for i in 0..4800 {
            let s = (i as f64 / 48000.0 * 1000.0 * 2.0 * std::f64::consts::PI).sin() as f32;
            samples.push(s);
            samples.push(s);
        }
        let buf = make_buf(&samples, 2, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.enabled = true;
        eq.settings.set_band(5, 6.0);
        eq.update_coefficients();
        let out = eq.process(&buf);
        assert_eq!(out.channels, 2);
        assert_eq!(out.num_frames, 4800);
    }

    // ---- Preset tests ----

    #[test]
    fn preset_names_list() {
        let names = EqSettings::preset_names();
        assert!(names.contains(&"rock"));
        assert!(names.contains(&"jazz"));
        assert!(names.contains(&"flat"));
        assert!(names.len() >= 9);
    }

    #[test]
    fn preset_rock() {
        let eq = EqSettings::preset("rock");
        assert!(eq.enabled);
        assert!(!eq.is_flat());
        assert!(eq.bands[0] > 0.0);
    }

    #[test]
    fn preset_flat_is_default() {
        let eq = EqSettings::preset("flat");
        assert!(eq.bands.iter().all(|b| *b == 0.0));
    }

    #[test]
    fn preset_unknown_returns_flat() {
        let eq = EqSettings::preset("nonexistent");
        assert!(eq.bands.iter().all(|b| *b == 0.0));
    }

    #[test]
    fn all_presets_valid_range() {
        for name in EqSettings::preset_names() {
            let eq = EqSettings::preset(name);
            for &b in &eq.bands {
                assert!(
                    (-12.0..=12.0).contains(&b),
                    "preset '{name}' band out of range: {b}"
                );
            }
        }
    }

    // ---- GainSmoother tests ----

    #[test]
    fn gain_smoother_converges() {
        let mut smoother = GainSmoother::new(0.3, 0.05);
        for _ in 0..50 {
            smoother.smooth(0.5);
        }
        assert!((smoother.current() - 0.5).abs() < 0.01);
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }

    // ---- Buffer alignment guard tests ----

    #[test]
    fn buf_to_f32_aligned_buffer() {
        // 8 bytes = exactly 2 f32 samples -- should work without issues
        let samples = [0.5f32, -0.25f32];
        let buf = make_buf(&samples, 1, 44100);
        let result = buf_to_f32_safe(&buf);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.5).abs() < f32::EPSILON);
        assert!((result[1] - (-0.25)).abs() < f32::EPSILON);
    }

    #[test]
    fn buf_to_f32_unaligned_buffer_truncates() {
        // Construct a buffer with 5 bytes (not a multiple of 4).
        // The guard should truncate the trailing byte and return 1 f32 sample.
        let sample = 0.75f32;
        let mut raw_bytes = sample.to_ne_bytes().to_vec();
        raw_bytes.push(0xAB); // extra trailing byte
        assert_eq!(raw_bytes.len(), 5);

        let buf = AudioBuffer {
            data: Bytes::from(raw_bytes),
            sample_format: SampleFormat::F32,
            channels: 1,
            sample_rate: 44100,
            num_frames: 1,
            timestamp: Duration::ZERO,
        };
        let result = buf_to_f32_safe(&buf);
        // Should have truncated the trailing byte, yielding 1 sample
        assert_eq!(result.len(), 1);
        assert!((result[0] - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn buf_to_f32_empty_buffer() {
        let buf = make_buf(&[], 1, 44100);
        let result = buf_to_f32_safe(&buf);
        assert!(result.is_empty());
    }
}
