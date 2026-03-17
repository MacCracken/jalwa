//! Local audio fingerprint similarity — uses tarang-ai directly for
//! fingerprint computation and matching without a daimon roundtrip.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tarang_ai::{AudioFingerprint, FingerprintConfig, compute_fingerprint, fingerprint_match};
use uuid::Uuid;

/// A fingerprint match result for a library item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintMatch {
    pub item_id: Uuid,
    pub score: f64,
    pub path: String,
}

/// Compute the audio fingerprint for a file by decoding its first ~30 seconds.
pub fn fingerprint_file(path: &Path) -> Result<AudioFingerprint> {
    let mut decoder = tarang_audio::FileDecoder::open_path(path)?;
    let config = FingerprintConfig::default();

    // Decode enough audio for a meaningful fingerprint (~30 seconds)
    let max_samples = config.sample_rate as usize * 30;
    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        match decoder.next_buffer() {
            Ok(buf) => {
                let samples: &[f32] = unsafe {
                    std::slice::from_raw_parts(
                        buf.data.as_ptr() as *const f32,
                        buf.data.len() / 4,
                    )
                };
                // Downmix to mono if stereo
                if buf.channels > 1 {
                    for chunk in samples.chunks(buf.channels as usize) {
                        let mono: f32 =
                            chunk.iter().sum::<f32>() / buf.channels as f32;
                        all_samples.push(mono);
                    }
                } else {
                    all_samples.extend_from_slice(samples);
                }
                if all_samples.len() >= max_samples {
                    all_samples.truncate(max_samples);
                    break;
                }
            }
            Err(tarang_core::TarangError::EndOfStream) => break,
            Err(e) => return Err(e.into()),
        }
    }

    if all_samples.is_empty() {
        anyhow::bail!("no audio samples decoded from {}", path.display());
    }

    let buf = tarang_core::AudioBuffer {
        data: bytes::Bytes::copy_from_slice(unsafe {
            std::slice::from_raw_parts(
                all_samples.as_ptr() as *const u8,
                all_samples.len() * 4,
            )
        }),
        sample_format: tarang_core::SampleFormat::F32,
        channels: 1,
        sample_rate: config.sample_rate,
        num_samples: all_samples.len(),
        timestamp: std::time::Duration::ZERO,
    };

    let fp = compute_fingerprint(&buf, &config)?;
    Ok(fp)
}

/// Find items in the library that are acoustically similar to a seed file.
///
/// Computes the fingerprint for `seed_path`, then compares against each
/// library item. Returns matches sorted by similarity score (descending),
/// filtered to `threshold` minimum similarity (0.0–1.0).
pub fn find_similar_local(
    library: &jalwa_core::Library,
    seed_path: &Path,
    max_results: usize,
    threshold: f64,
) -> Vec<FingerprintMatch> {
    let seed_fp = match fingerprint_file(seed_path) {
        Ok(fp) => fp,
        Err(e) => {
            tracing::warn!("failed to fingerprint seed: {e}");
            return Vec::new();
        }
    };

    let mut matches: Vec<FingerprintMatch> = library
        .items
        .iter()
        .filter(|item| item.path != seed_path)
        .filter_map(|item| {
            let fp = fingerprint_file(&item.path).ok()?;
            let score = fingerprint_match(&seed_fp, &fp);
            if score >= threshold {
                Some(FingerprintMatch {
                    item_id: item.id,
                    score,
                    path: item.path.display().to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(|a, b| b.score.total_cmp(&a.score));
    matches.truncate(max_results);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_match_serialization() {
        let m = FingerprintMatch {
            item_id: Uuid::new_v4(),
            score: 0.85,
            path: "/music/song.flac".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let parsed: FingerprintMatch = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "/music/song.flac");
        assert!((parsed.score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn fingerprint_nonexistent_file() {
        let result = fingerprint_file(Path::new("/nonexistent/file.flac"));
        assert!(result.is_err());
    }

    #[test]
    fn find_similar_empty_library() {
        let lib = jalwa_core::Library::new();
        let results = find_similar_local(&lib, Path::new("/some/seed.flac"), 5, 0.5);
        assert!(results.is_empty());
    }
}
