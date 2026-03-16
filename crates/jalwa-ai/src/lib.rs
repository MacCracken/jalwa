//! jalwa-ai — AI features for the Jalwa media player
//!
//! Smart playlists, content-based recommendations, transcription routing via hoosh,
//! and media analysis powered by tarang-ai.

use jalwa_core::{Library, MediaType, Playlist};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Recommendation reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationReason {
    SameArtist,
    SameAlbum,
    SameGenre,
    SimilarDuration,
    FrequentlyPlayed,
    RecentlyAdded,
    Tagged(String),
}

impl std::fmt::Display for RecommendationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SameArtist => write!(f, "same artist"),
            Self::SameAlbum => write!(f, "same album"),
            Self::SameGenre => write!(f, "same genre"),
            Self::SimilarDuration => write!(f, "similar duration"),
            Self::FrequentlyPlayed => write!(f, "frequently played"),
            Self::RecentlyAdded => write!(f, "recently added"),
            Self::Tagged(tag) => write!(f, "tagged '{tag}'"),
        }
    }
}

/// A recommended media item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub item_id: Uuid,
    pub score: f32,
    pub reasons: Vec<RecommendationReason>,
}

/// Generate recommendations based on a seed item
pub fn recommend(library: &Library, seed_id: Uuid, max_results: usize) -> Vec<Recommendation> {
    let seed = match library.find_by_id(seed_id) {
        Some(item) => item,
        None => return Vec::new(),
    };

    let mut recs: Vec<Recommendation> = library
        .items
        .iter()
        .filter(|item| item.id != seed_id)
        .map(|item| {
            let mut score: f32 = 0.0;
            let mut reasons = Vec::new();

            // Same artist
            if let (Some(seed_artist), Some(item_artist)) = (&seed.artist, &item.artist)
                && seed_artist.to_lowercase() == item_artist.to_lowercase()
            {
                score += 30.0;
                reasons.push(RecommendationReason::SameArtist);
            }

            // Same album
            if let (Some(seed_album), Some(item_album)) = (&seed.album, &item.album)
                && seed_album.to_lowercase() == item_album.to_lowercase()
            {
                score += 20.0;
                reasons.push(RecommendationReason::SameAlbum);
            }

            // Similar duration (within 30%)
            if let (Some(seed_dur), Some(item_dur)) = (seed.duration, item.duration) {
                let ratio = item_dur.as_secs_f64() / seed_dur.as_secs_f64().max(1.0);
                if (0.7..=1.3).contains(&ratio) {
                    score += 10.0;
                    reasons.push(RecommendationReason::SimilarDuration);
                }
            }

            // Same type (audio with audio, video with video)
            if item.media_type == seed.media_type {
                score += 5.0;
            }

            // Shared tags
            for tag in &seed.tags {
                if item
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase() == tag.to_lowercase())
                {
                    score += 15.0;
                    reasons.push(RecommendationReason::Tagged(tag.clone()));
                }
            }

            // Frequently played bonus
            if item.play_count >= 5 {
                score += 5.0;
                reasons.push(RecommendationReason::FrequentlyPlayed);
            }

            Recommendation {
                item_id: item.id,
                score,
                reasons,
            }
        })
        .filter(|r| r.score > 0.0)
        .collect();

    recs.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recs.truncate(max_results);
    recs
}

/// Smart playlist criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmartCriteria {
    Artist(String),
    Album(String),
    Tag(String),
    MediaType(MediaType),
    MinDuration(Duration),
    MaxDuration(Duration),
    MinPlayCount(u32),
    MinRating(u8),
    RecentlyAdded { days: u32 },
    RecentlyPlayed { days: u32 },
}

/// Generate a smart playlist from criteria
pub fn generate_smart_playlist(
    library: &Library,
    name: &str,
    criteria: &[SmartCriteria],
) -> Playlist {
    let mut playlist = Playlist::new(name);
    playlist.is_smart = true;

    let now = chrono::Utc::now();

    for item in &library.items {
        let matches = criteria.iter().all(|c| match c {
            SmartCriteria::Artist(a) => item
                .artist
                .as_ref()
                .is_some_and(|ia| ia.to_lowercase().contains(&a.to_lowercase())),
            SmartCriteria::Album(a) => item
                .album
                .as_ref()
                .is_some_and(|ia| ia.to_lowercase().contains(&a.to_lowercase())),
            SmartCriteria::Tag(t) => item
                .tags
                .iter()
                .any(|it| it.to_lowercase() == t.to_lowercase()),
            SmartCriteria::MediaType(mt) => item.media_type == *mt,
            SmartCriteria::MinDuration(d) => item.duration.is_some_and(|id| id >= *d),
            SmartCriteria::MaxDuration(d) => item.duration.is_some_and(|id| id <= *d),
            SmartCriteria::MinPlayCount(n) => item.play_count >= *n,
            SmartCriteria::MinRating(r) => item.rating.is_some_and(|ir| ir >= *r),
            SmartCriteria::RecentlyAdded { days } => {
                let cutoff = now - chrono::Duration::days(*days as i64);
                item.added_at >= cutoff
            }
            SmartCriteria::RecentlyPlayed { days } => {
                let cutoff = now - chrono::Duration::days(*days as i64);
                item.last_played.is_some_and(|lp| lp >= cutoff)
            }
        });

        if matches {
            playlist.add(item.id);
        }
    }

    playlist
}

/// Analyze the library and return insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInsights {
    pub top_artists: Vec<(String, u32)>,
    pub most_played: Vec<(Uuid, u32)>,
    pub total_listen_time: Duration,
    pub avg_track_duration: Duration,
    pub genre_distribution: Vec<(String, usize)>,
}

pub fn analyze_library(library: &Library) -> LibraryInsights {
    use std::collections::HashMap;

    let mut artist_counts: HashMap<String, u32> = HashMap::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut total_listen: Duration = Duration::ZERO;
    let mut total_duration: Duration = Duration::ZERO;
    let mut play_counts: Vec<(Uuid, u32)> = Vec::new();

    for item in &library.items {
        if let Some(artist) = &item.artist {
            *artist_counts.entry(artist.clone()).or_default() += 1;
        }
        for tag in &item.tags {
            *tag_counts.entry(tag.clone()).or_default() += 1;
        }
        if let Some(dur) = item.duration {
            total_duration += dur;
            total_listen += dur * item.play_count;
        }
        if item.play_count > 0 {
            play_counts.push((item.id, item.play_count));
        }
    }

    let mut top_artists: Vec<(String, u32)> = artist_counts.into_iter().collect();
    top_artists.sort_by(|a, b| b.1.cmp(&a.1));
    top_artists.truncate(10);

    play_counts.sort_by(|a, b| b.1.cmp(&a.1));
    play_counts.truncate(10);

    let mut genre_distribution: Vec<(String, usize)> = tag_counts.into_iter().collect();
    genre_distribution.sort_by(|a, b| b.1.cmp(&a.1));

    let avg_track_duration = if library.items.is_empty() {
        Duration::ZERO
    } else {
        total_duration / library.items.len() as u32
    };

    LibraryInsights {
        top_artists,
        most_played: play_counts,
        total_listen_time: total_listen,
        avg_track_duration,
        genre_distribution,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::*;
    use std::path::PathBuf;
    use tarang_core::{AudioCodec, ContainerFormat};

    fn make_item(title: &str, artist: &str, duration_secs: u64) -> MediaItem {
        MediaItem {
            id: Uuid::new_v4(),
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: Some("Album".to_string()),
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
    fn recommend_same_artist() {
        let mut lib = Library::new();
        let mut seed = make_item("Song A", "Artist 1", 200);
        let seed_id = seed.id;
        lib.add_item(seed);

        let mut same_artist = make_item("Song B", "Artist 1", 180);
        lib.add_item(same_artist);

        let diff_artist = make_item("Song C", "Artist 2", 200);
        lib.add_item(diff_artist);

        let recs = recommend(&lib, seed_id, 10);
        assert!(!recs.is_empty());
        // Same artist should score highest
        assert!(
            recs[0]
                .reasons
                .iter()
                .any(|r| matches!(r, RecommendationReason::SameArtist))
        );
    }

    #[test]
    fn recommend_empty_library() {
        let lib = Library::new();
        let recs = recommend(&lib, Uuid::new_v4(), 10);
        assert!(recs.is_empty());
    }

    #[test]
    fn recommend_nonexistent_seed() {
        let mut lib = Library::new();
        lib.add_item(make_item("Song", "Artist", 200));
        let recs = recommend(&lib, Uuid::new_v4(), 10);
        assert!(recs.is_empty());
    }

    #[test]
    fn recommend_with_tags() {
        let mut lib = Library::new();
        let mut seed = make_item("Song A", "X", 200);
        seed.tags = vec!["rock".to_string()];
        let seed_id = seed.id;
        lib.add_item(seed);

        let mut tagged = make_item("Song B", "Y", 180);
        tagged.tags = vec!["rock".to_string()];
        lib.add_item(tagged);

        let recs = recommend(&lib, seed_id, 10);
        assert!(
            recs[0]
                .reasons
                .iter()
                .any(|r| matches!(r, RecommendationReason::Tagged(_)))
        );
    }

    #[test]
    fn recommend_max_results() {
        let mut lib = Library::new();
        let mut seed = make_item("Seed", "Artist", 200);
        let seed_id = seed.id;
        lib.add_item(seed);

        for i in 0..20 {
            lib.add_item(make_item(&format!("Song {i}"), "Artist", 200));
        }

        let recs = recommend(&lib, seed_id, 5);
        assert_eq!(recs.len(), 5);
    }

    #[test]
    fn smart_playlist_by_artist() {
        let mut lib = Library::new();
        lib.add_item(make_item("Song 1", "Queen", 200));
        lib.add_item(make_item("Song 2", "Queen", 180));
        lib.add_item(make_item("Song 3", "Beatles", 200));

        let pl = generate_smart_playlist(
            &lib,
            "Queen Songs",
            &[SmartCriteria::Artist("queen".to_string())],
        );
        assert_eq!(pl.len(), 2);
        assert!(pl.is_smart);
    }

    #[test]
    fn smart_playlist_by_tag() {
        let mut lib = Library::new();
        let mut item = make_item("Song", "A", 200);
        item.tags = vec!["jazz".to_string()];
        lib.add_item(item);
        lib.add_item(make_item("Other", "B", 200));

        let pl = generate_smart_playlist(&lib, "Jazz", &[SmartCriteria::Tag("jazz".to_string())]);
        assert_eq!(pl.len(), 1);
    }

    #[test]
    fn smart_playlist_by_duration() {
        let mut lib = Library::new();
        lib.add_item(make_item("Short", "A", 60));
        lib.add_item(make_item("Long", "B", 600));

        let pl = generate_smart_playlist(
            &lib,
            "Long Tracks",
            &[SmartCriteria::MinDuration(Duration::from_secs(300))],
        );
        assert_eq!(pl.len(), 1);
    }

    #[test]
    fn smart_playlist_multiple_criteria() {
        let mut lib = Library::new();
        let mut item = make_item("Song", "Queen", 200);
        item.tags = vec!["rock".to_string()];
        lib.add_item(item);

        let no_tag = make_item("Other", "Queen", 200);
        lib.add_item(no_tag);

        let pl = generate_smart_playlist(
            &lib,
            "Queen Rock",
            &[
                SmartCriteria::Artist("queen".to_string()),
                SmartCriteria::Tag("rock".to_string()),
            ],
        );
        assert_eq!(pl.len(), 1); // Only the one with both
    }

    #[test]
    fn analyze_empty_library() {
        let lib = Library::new();
        let insights = analyze_library(&lib);
        assert!(insights.top_artists.is_empty());
        assert!(insights.most_played.is_empty());
        assert_eq!(insights.total_listen_time, Duration::ZERO);
    }

    #[test]
    fn analyze_with_data() {
        let mut lib = Library::new();
        let mut item1 = make_item("Song 1", "Queen", 200);
        item1.play_count = 10;
        item1.tags = vec!["rock".to_string()];
        lib.add_item(item1);

        let mut item2 = make_item("Song 2", "Queen", 300);
        item2.play_count = 5;
        lib.add_item(item2);

        lib.add_item(make_item("Song 3", "Beatles", 180));

        let insights = analyze_library(&lib);
        assert_eq!(insights.top_artists[0].0, "Queen");
        assert_eq!(insights.top_artists[0].1, 2);
        assert_eq!(insights.most_played.len(), 2);
        assert!(insights.total_listen_time > Duration::ZERO);
        assert!(insights.avg_track_duration > Duration::ZERO);
    }

    #[test]
    fn recommendation_reason_display() {
        assert_eq!(RecommendationReason::SameArtist.to_string(), "same artist");
        assert_eq!(
            RecommendationReason::Tagged("rock".to_string()).to_string(),
            "tagged 'rock'"
        );
    }
}
