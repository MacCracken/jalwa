//! Audio DSP — equalizer and volume normalization.
//!
//! All functions operate on interleaved F32 `AudioBuffer`s from tarang-core.

use tarang_core::AudioBuffer;

// ---- Volume Normalization / ReplayGain ----

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
const TARGET_RMS: f32 = 0.125;

/// Analyze a buffer's loudness and compute normalization gain.
pub fn analyze_loudness(buf: &AudioBuffer) -> LoudnessInfo {
    let samples = buf_to_f32(buf);
    if samples.is_empty() {
        return LoudnessInfo {
            rms: 0.0,
            peak: 0.0,
            gain: 1.0,
        };
    }

    let mut sum_sq: f64 = 0.0;
    let mut peak: f32 = 0.0;
    for &s in samples.iter() {
        sum_sq += (s as f64) * (s as f64);
        let abs: f32 = s.abs();
        if abs > peak {
            peak = abs;
        }
    }

    let rms = (sum_sq / samples.len() as f64).sqrt() as f32;

    // Compute gain to reach target RMS, clamped to avoid extreme amplification
    let gain = if rms > 1e-6 {
        (TARGET_RMS / rms).clamp(0.1, 10.0)
    } else {
        1.0
    };

    LoudnessInfo { rms, peak, gain }
}

/// Apply normalization gain to a buffer, with peak limiting to prevent clipping.
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

// ---- 10-Band Graphic Equalizer ----

/// Standard 10-band ISO center frequencies (Hz).
pub const EQ_BANDS: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Equalizer settings: gain per band in dB (-12.0 to +12.0).
#[derive(Debug, Clone)]
pub struct EqSettings {
    /// Gain per band in dB. Length must be 10.
    pub bands: [f32; 10],
    pub enabled: bool,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            bands: [0.0; 10],
            enabled: false,
        }
    }
}

impl EqSettings {
    /// Flat EQ (all bands at 0 dB).
    pub fn flat() -> Self {
        Self::default()
    }

    /// Check if all bands are at 0 dB (no processing needed).
    pub fn is_flat(&self) -> bool {
        !self.enabled || self.bands.iter().all(|b| b.abs() < 0.01)
    }

    /// Set a specific band's gain in dB, clamped to ±12.
    pub fn set_band(&mut self, band: usize, gain_db: f32) {
        if band < 10 {
            self.bands[band] = gain_db.clamp(-12.0, 12.0);
        }
    }

    /// Load a named preset.
    pub fn preset(name: &str) -> Self {
        //                       31   62  125  250  500   1k   2k   4k   8k  16k
        let bands = match name {
            "rock" => [4.0, 3.0, 1.0, -1.0, -2.0, 0.0, 2.0, 3.0, 4.0, 4.0],
            "pop" => [-1.0, 1.0, 3.0, 4.0, 3.0, 0.0, -1.0, 0.0, 1.0, 2.0],
            "jazz" => [2.0, 1.0, 0.0, 1.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0],
            "classical" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, -3.0, -2.0, 0.0],
            "bass" => [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "treble" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 5.0, 6.0],
            "vocal" => [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0],
            "electronic" => [5.0, 4.0, 1.0, 0.0, -2.0, 0.0, 1.0, 3.0, 4.0, 5.0],
            "acoustic" => [2.0, 1.0, 0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0],
            _ => [0.0; 10],
        };
        Self {
            bands,
            enabled: true,
        }
    }

    /// List all available preset names.
    pub fn preset_names() -> &'static [&'static str] {
        &[
            "flat",
            "rock",
            "pop",
            "jazz",
            "classical",
            "bass",
            "treble",
            "vocal",
            "electronic",
            "acoustic",
        ]
    }

    /// Get the band index for a display name.
    pub fn band_name(band: usize) -> &'static str {
        match band {
            0 => "31 Hz",
            1 => "62 Hz",
            2 => "125 Hz",
            3 => "250 Hz",
            4 => "500 Hz",
            5 => "1 kHz",
            6 => "2 kHz",
            7 => "4 kHz",
            8 => "8 kHz",
            9 => "16 kHz",
            _ => "?",
        }
    }
}

/// Biquad filter coefficients for a single band.
#[derive(Debug, Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

/// Biquad filter state (per channel).
#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

/// Compute peaking EQ biquad coefficients.
/// `freq`: center frequency, `gain_db`: boost/cut, `q`: quality factor, `sr`: sample rate.
fn peaking_eq(freq: f32, gain_db: f32, q: f32, sr: f32) -> BiquadCoeffs {
    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * freq / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Apply a biquad filter to a single sample, updating state.
fn biquad_process(c: &BiquadCoeffs, s: &mut BiquadState, input: f32) -> f32 {
    let output = c.b0 * input + c.b1 * s.x1 + c.b2 * s.x2 - c.a1 * s.y1 - c.a2 * s.y2;
    s.x2 = s.x1;
    s.x1 = input;
    s.y2 = s.y1;
    s.y1 = output;
    output
}

/// Maximum number of channels supported by the EQ.
const MAX_EQ_CHANNELS: usize = 8;

/// 10-band graphic equalizer processor.
///
/// Holds per-band biquad filter state for each channel (up to 8).
/// Call `process()` on each decoded buffer in the decode loop.
pub struct Equalizer {
    coeffs: [BiquadCoeffs; 10],
    /// Per-band, per-channel state. Indexed as [band][channel].
    state: [[BiquadState; MAX_EQ_CHANNELS]; 10],
    sample_rate: u32,
    pub settings: EqSettings,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        let mut eq = Self {
            coeffs: [BiquadCoeffs {
                b0: 1.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
            }; 10],
            state: [[BiquadState::default(); MAX_EQ_CHANNELS]; 10],
            sample_rate,
            settings: EqSettings::default(),
        };
        eq.update_coefficients();
        eq
    }

    /// Recompute filter coefficients from current settings.
    pub fn update_coefficients(&mut self) {
        let q = 1.4; // Standard Q for graphic EQ
        for (i, &freq) in EQ_BANDS.iter().enumerate() {
            self.coeffs[i] = peaking_eq(freq, self.settings.bands[i], q, self.sample_rate as f32);
        }
    }

    /// Reset filter state (call on seek or track change).
    pub fn reset(&mut self) {
        self.state = [[BiquadState::default(); MAX_EQ_CHANNELS]; 10];
    }

    /// Process an audio buffer through the 10-band EQ in-place.
    pub fn process(&mut self, buf: &AudioBuffer) -> AudioBuffer {
        if !self.settings.enabled || self.settings.is_flat() {
            return buf.clone();
        }

        // Update coefficients if sample rate changed
        if buf.sample_rate != self.sample_rate {
            self.sample_rate = buf.sample_rate;
            self.update_coefficients();
        }

        let samples = buf_to_f32(buf);
        let channels = buf.channels as usize;
        let active_channels = channels.min(MAX_EQ_CHANNELS);
        let mut output = samples.to_vec();

        // Apply each band's biquad filter in series
        for band in 0..10 {
            if self.settings.bands[band].abs() < 0.01 {
                continue; // Skip flat bands
            }
            let coeffs = &self.coeffs[band];
            for frame in 0..(output.len() / channels) {
                for ch in 0..active_channels {
                    let idx = frame * channels + ch;
                    output[idx] = biquad_process(coeffs, &mut self.state[band][ch], output[idx]);
                }
            }
        }

        f32_to_buf(&output, buf)
    }
}

// ---- Helpers ----

/// Safely interpret an AudioBuffer's bytes as `&[f32]`.
///
/// If the underlying `Bytes` is not 4-byte aligned we copy into an
/// aligned temporary. The returned `Cow` avoids the copy when alignment
/// is already correct (the common path).
pub(crate) fn buf_to_f32_safe(buf: &AudioBuffer) -> std::borrow::Cow<'_, [f32]> {
    if buf.data.is_empty() {
        return std::borrow::Cow::Borrowed(&[]);
    }
    match bytemuck::try_cast_slice::<u8, f32>(&buf.data) {
        Ok(s) => std::borrow::Cow::Borrowed(s),
        Err(_) => {
            // Fallback: copy into an aligned Vec
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
fn buf_to_f32(buf: &AudioBuffer) -> std::borrow::Cow<'_, [f32]> {
    buf_to_f32_safe(buf)
}

fn f32_to_buf(samples: &[f32], template: &AudioBuffer) -> AudioBuffer {
    let bytes: &[u8] = bytemuck::cast_slice(samples);
    AudioBuffer {
        data: bytes::Bytes::copy_from_slice(bytes),
        sample_format: template.sample_format,
        channels: template.channels,
        sample_rate: template.sample_rate,
        num_samples: template.num_samples,
        timestamp: template.timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::time::Duration;
    use tarang_core::SampleFormat;

    fn make_buf(samples: &[f32], channels: u16, sample_rate: u32) -> AudioBuffer {
        let bytes: &[u8] = bytemuck::cast_slice(samples);
        AudioBuffer {
            data: Bytes::copy_from_slice(bytes),
            sample_format: SampleFormat::F32,
            channels,
            sample_rate,
            num_samples: samples.len() / channels as usize,
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
        assert_eq!(info.gain, 1.0); // no amplification of silence
    }

    #[test]
    fn loudness_full_scale_sine() {
        let samples = make_sine(440.0, 44100, 44100);
        let buf = make_buf(&samples, 1, 44100);
        let info = analyze_loudness(&buf);
        // Sine wave RMS ≈ 0.707
        assert!(info.rms > 0.5 && info.rms < 0.8, "rms={}", info.rms);
        assert!(info.peak > 0.99, "peak={}", info.peak);
        // Gain should attenuate since RMS >> TARGET_RMS
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
        let result = normalize(&buf, 3.0); // 3x gain would clip
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
        let mut eq = EqSettings::default();
        eq.enabled = true;
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
        eq.set_band(99, 6.0); // should not panic
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
        eq.settings.enabled = true; // enabled but flat
        let out = eq.process(&buf);
        // Flat EQ should return unchanged data
        assert_eq!(out.data, buf.data);
    }

    #[test]
    fn eq_disabled_passthrough() {
        let samples = make_sine(1000.0, 48000, 4800);
        let buf = make_buf(&samples, 1, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.set_band(5, 12.0); // boost 1kHz but disabled
        let out = eq.process(&buf);
        assert_eq!(out.data, buf.data);
    }

    #[test]
    fn eq_boost_changes_signal() {
        let samples = make_sine(1000.0, 48000, 4800);
        let buf = make_buf(&samples, 1, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.enabled = true;
        eq.settings.set_band(5, 12.0); // boost 1kHz by 12dB
        eq.update_coefficients();
        let out = eq.process(&buf);
        // Output should differ from input
        assert_ne!(out.data, buf.data);
        // RMS should be higher (boosted)
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
        eq.settings.set_band(5, -12.0); // cut 1kHz
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
        eq.state[0][0].y1 = 1.0;
        eq.reset();
        assert_eq!(eq.state[0][0].y1, 0.0);
    }

    #[test]
    fn eq_stereo() {
        let mut samples = Vec::new();
        for i in 0..4800 {
            let s = (i as f64 / 48000.0 * 1000.0 * 2.0 * std::f64::consts::PI).sin() as f32;
            samples.push(s); // L
            samples.push(s); // R
        }
        let buf = make_buf(&samples, 2, 48000);
        let mut eq = Equalizer::new(48000);
        eq.settings.enabled = true;
        eq.settings.set_band(5, 6.0);
        eq.update_coefficients();
        let out = eq.process(&buf);
        assert_eq!(out.channels, 2);
        assert_eq!(out.num_samples, 4800);
    }

    #[test]
    fn peaking_eq_unity_at_zero_gain() {
        let c = peaking_eq(1000.0, 0.0, 1.4, 48000.0);
        // At 0 dB gain, filter should be near unity: b0≈1, b1≈a1, b2≈a2
        assert!((c.b0 - 1.0).abs() < 0.01, "b0={}", c.b0);
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
        // Rock has bass boost
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
                    b >= -12.0 && b <= 12.0,
                    "preset '{name}' band out of range: {b}"
                );
            }
        }
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }
}
