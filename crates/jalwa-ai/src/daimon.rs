//! Daimon (agent-runtime) integration for Jalwa
//!
//! Connects jalwa to the AGNOS agent orchestrator for:
//! - Agent registration with audio modality
//! - RAG ingestion of library metadata for NL queries
//! - Vector store for fingerprint-based similarity recommendations
//! - LLM-powered recommendations via hoosh ("find me something chill")

use anyhow::{Context, Result, bail};
use jalwa_core::{Library, MediaItem};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the daimon (agent-runtime) connection.
#[derive(Debug, Clone)]
pub struct DaimonConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl Default for DaimonConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("DAIMON_URL")
                .unwrap_or_else(|_| "http://localhost:8090".to_string()),
            api_key: std::env::var("DAIMON_API_KEY").ok(),
        }
    }
}

/// Configuration for hoosh LLM-powered features.
#[derive(Debug, Clone)]
pub struct HooshConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
}

impl Default for HooshConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("HOOSH_URL")
                .unwrap_or_else(|_| "http://localhost:8088".to_string()),
            api_key: std::env::var("HOOSH_API_KEY").ok(),
            model: std::env::var("HOOSH_MODEL")
                .unwrap_or_else(|_| "llama3".to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Daimon Client
// ---------------------------------------------------------------------------

/// Client for integrating jalwa with daimon services.
pub struct DaimonClient {
    config: DaimonConfig,
    http: reqwest::Client,
}

impl DaimonClient {
    pub fn new(config: DaimonConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("HTTP client error: {e}"))?;
        Ok(Self { config, http })
    }

    fn auth_header(&self) -> Option<String> {
        self.config.api_key.as_ref().map(|k| format!("Bearer {k}"))
    }

    /// Register jalwa as a media playback agent with daimon.
    pub async fn register_agent(&self) -> Result<()> {
        let body = serde_json::json!({
            "name": "jalwa",
            "id": "jalwa-media-player",
            "domain": "media",
            "capabilities": ["audio_playback", "library_management", "smart_playlists", "recommendations", "media_search"],
            "metadata": {
                "modalities_input": ["audio"],
                "modalities_output": ["text", "structured_data"],
                "version": env!("CARGO_PKG_VERSION"),
                "runtime": "native-binary",
                "port": 8093,
            }
        });

        let url = format!("{}/v1/agents/register", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("agent registration failed: {e}"))?;

        if resp.status().is_success() {
            info!("Registered jalwa as media playback agent with daimon");
        } else {
            let status = resp.status();
            warn!(%status, "Agent registration returned non-success");
        }

        Ok(())
    }

    /// Ingest a library item's metadata into the RAG pipeline.
    pub async fn ingest_item(&self, item: &MediaItem) -> Result<()> {
        let text = format_item_for_rag(item);

        let body = serde_json::json!({
            "text": text,
            "agent_id": "jalwa",
            "metadata": {
                "source": "jalwa",
                "item_id": item.id.to_string(),
                "media_type": format!("{:?}", item.media_type),
            }
        });

        let url = format!("{}/v1/rag/ingest", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("RAG ingest failed: {e}"))?;

        if !resp.status().is_success() {
            warn!(status = %resp.status(), "RAG ingest returned non-success");
        }

        Ok(())
    }

    /// Bulk ingest all library items into RAG.
    pub async fn ingest_library(&self, library: &Library) -> Result<usize> {
        let mut count = 0;
        for item in &library.items {
            if self.ingest_item(item).await.is_ok() {
                count += 1;
            }
        }
        info!(count, total = library.items.len(), "Ingested library into RAG");
        Ok(count)
    }

    /// Query RAG for media matching a natural-language description.
    pub async fn query_media(&self, query: &str, top_k: usize) -> Result<Vec<RagResult>> {
        let body = serde_json::json!({
            "query": query,
            "top_k": top_k,
            "agent_id": "jalwa",
        });

        let url = format!("{}/v1/rag/query", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("RAG query failed: {e}"))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let result: RagQueryResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("parse RAG response: {e}"))?;

        Ok(result.results)
    }

    /// Index a media item's audio fingerprint via tarang's MCP tool through daimon.
    ///
    /// Delegates to `tarang_fingerprint_index` — tarang handles decode, fingerprinting,
    /// and vector store insertion. This is the proper AGNOS architecture: jalwa asks
    /// daimon to call tarang rather than importing tarang-ai directly.
    pub async fn index_fingerprint(&self, path: &str) -> Result<()> {
        let body = serde_json::json!({
            "name": "tarang_fingerprint_index",
            "arguments": { "path": path },
        });

        let url = format!("{}/v1/mcp/tools/call", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("fingerprint index failed: {e}"))?;

        if resp.status().is_success() {
            info!(path, "Indexed fingerprint via tarang MCP tool");
        }

        Ok(())
    }

    /// Search for similar media via tarang's MCP tool through daimon.
    pub async fn search_similar(&self, path: &str, top_k: usize) -> Result<Vec<SimilarMedia>> {
        let body = serde_json::json!({
            "name": "tarang_search_similar",
            "arguments": { "path": path, "top_k": top_k },
        });

        let url = format!("{}/v1/mcp/tools/call", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("similarity search failed: {e}"))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        // Parse the MCP tool response which wraps results in content blocks
        let result: serde_json::Value = resp.json().await
            .map_err(|e| anyhow::anyhow!("parse search response: {e}"))?;

        // Try to extract results from the MCP response text
        let text = result["content"][0]["text"].as_str().unwrap_or("[]");
        let results: Vec<SimilarMedia> = serde_json::from_str(text).unwrap_or_default();
        Ok(results)
    }

    /// Route audio for transcription via tarang's MCP tool through daimon.
    pub async fn transcribe(&self, path: &str, language: Option<&str>) -> Result<TranscriptionResult> {
        let mut args = serde_json::json!({ "path": path });
        if let Some(lang) = language {
            args["language"] = serde_json::Value::String(lang.to_string());
        }

        let body = serde_json::json!({
            "name": "tarang_transcribe",
            "arguments": args,
        });

        let url = format!("{}/v1/mcp/tools/call", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("transcription request failed: {e}"))?;

        if !resp.status().is_success() {
            bail!("transcription returned {}", resp.status());
        }

        let result: serde_json::Value = resp.json().await
            .map_err(|e| anyhow::anyhow!("parse transcription response: {e}"))?;

        let text = result["content"][0]["text"].as_str().unwrap_or("");

        if let Ok(tr) = serde_json::from_str::<TranscriptionResult>(text) {
            Ok(tr)
        } else {
            Ok(TranscriptionResult {
                text: text.to_string(),
                language: language.unwrap_or("unknown").to_string(),
                segments: Vec::new(),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Hoosh LLM Client
// ---------------------------------------------------------------------------

/// Client for LLM-powered features via hoosh.
pub struct HooshLlmClient {
    config: HooshConfig,
    http: reqwest::Client,
}

impl HooshLlmClient {
    pub fn new(config: HooshConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| anyhow::anyhow!("HTTP client error: {e}"))?;
        Ok(Self { config, http })
    }

    /// Get LLM-powered recommendations based on a natural language prompt.
    ///
    /// Sends library context + user prompt to hoosh for semantic matching.
    /// E.g., "find me something chill for working" or "upbeat songs for running"
    pub async fn llm_recommend(
        &self,
        library: &Library,
        prompt: &str,
    ) -> Result<LlmRecommendation> {
        let library_context = build_library_context(library);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [{
                "role": "user",
                "content": format!(
                    "You are a music recommendation engine. Given the user's media library and their request, \
                     suggest tracks from the library that match.\n\n\
                     Respond with JSON: {{\"suggestions\": [{{\"title\": \"...\", \"artist\": \"...\", \"reason\": \"...\"}}], \"mood\": \"...\"}}\n\n\
                     Library:\n{library_context}\n\nUser request: {prompt}\n\nRespond with only valid JSON."
                ),
            }],
            "temperature": 0.5,
            "max_tokens": 512,
        });

        let url = format!("{}/v1/chat/completions", self.config.endpoint);
        let mut req = self.http.post(&url).json(&body);
        if let Some(key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("hoosh request failed: {e}"))?;

        if !resp.status().is_success() {
            bail!("hoosh returned {}", resp.status());
        }

        let result: serde_json::Value = resp.json().await
            .map_err(|e| anyhow::anyhow!("parse LLM response: {e}"))?;

        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        parse_llm_recommendation(&content)
    }

}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A media file similar to a query fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarMedia {
    pub path: String,
    pub score: f64,
    pub metadata: serde_json::Value,
}

/// A RAG query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResult {
    pub text: String,
    pub relevance: f64,
}

/// LLM-generated recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRecommendation {
    pub suggestions: Vec<LlmSuggestion>,
    pub mood: Option<String>,
}

/// A single LLM suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSuggestion {
    pub title: String,
    pub artist: Option<String>,
    pub reason: String,
}

/// Transcription result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: String,
    pub segments: Vec<TranscriptionSegment>,
}

/// A timed transcription segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct VectorSearchResponse {
    results: Vec<VectorSearchResult>,
}

#[derive(Debug, Deserialize)]
struct VectorSearchResult {
    content: String,
    score: f64,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RagQueryResponse {
    results: Vec<RagResult>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format a media item for RAG ingestion.
fn format_item_for_rag(item: &MediaItem) -> String {
    let mut parts = vec![
        format!("Title: {}", item.title),
        format!("Path: {}", item.path.display()),
        format!("Format: {}", item.format),
        format!("Type: {:?}", item.media_type),
    ];

    if let Some(artist) = &item.artist {
        parts.push(format!("Artist: {artist}"));
    }
    if let Some(album) = &item.album {
        parts.push(format!("Album: {album}"));
    }
    if let Some(dur) = item.duration {
        let mins = dur.as_secs() / 60;
        let secs = dur.as_secs() % 60;
        parts.push(format!("Duration: {}:{:02}", mins, secs));
    }
    if let Some(codec) = &item.audio_codec {
        parts.push(format!("Codec: {codec}"));
    }
    if !item.tags.is_empty() {
        parts.push(format!("Tags: {}", item.tags.join(", ")));
    }
    if item.play_count > 0 {
        parts.push(format!("Play count: {}", item.play_count));
    }
    if let Some(rating) = item.rating {
        parts.push(format!("Rating: {}/5", rating));
    }

    parts.join("\n")
}

/// Build a compact library context for LLM prompts.
fn build_library_context(library: &Library) -> String {
    library.items.iter()
        .take(100) // Cap at 100 items to stay within token limits
        .map(|item| {
            let artist = item.artist.as_deref().unwrap_or("Unknown");
            let dur = item.duration
                .map(|d| format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60))
                .unwrap_or_else(|| "?".to_string());
            let tags = if item.tags.is_empty() { String::new() } else { format!(" [{}]", item.tags.join(", ")) };
            format!("- {} — {} ({}){}", item.title, artist, dur, tags)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse LLM response into an LlmRecommendation.
fn parse_llm_recommendation(response: &str) -> Result<LlmRecommendation> {
    // Try parsing as JSON first
    if let Ok(rec) = serde_json::from_str::<LlmRecommendation>(response) {
        return Ok(rec);
    }

    // Try extracting JSON from markdown block
    let json_str = response
        .find('{')
        .and_then(|start| response.rfind('}').map(|end| &response[start..=end]));

    if let Some(json) = json_str {
        if let Ok(rec) = serde_json::from_str::<LlmRecommendation>(json) {
            return Ok(rec);
        }
    }

    // Fallback: wrap raw text as a single suggestion
    Ok(LlmRecommendation {
        suggestions: vec![LlmSuggestion {
            title: response.trim().to_string(),
            artist: None,
            reason: "LLM response".to_string(),
        }],
        mood: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::*;
    use std::path::PathBuf;
    use std::time::Duration;
    use tarang_core::{AudioCodec, ContainerFormat};

    fn make_item(title: &str, artist: &str, duration_secs: u64) -> MediaItem {
        MediaItem {
            id: uuid::Uuid::new_v4(),
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
            art_mime: None,
            art_data: None,
        }
    }

    #[test]
    fn daimon_config_defaults() {
        let config = DaimonConfig::default();
        assert!(config.endpoint.contains("8090"));
    }

    #[test]
    fn hoosh_config_defaults() {
        let config = HooshConfig::default();
        assert!(config.endpoint.contains("8088"));
    }

    #[test]
    fn format_item_rag_includes_metadata() {
        let mut item = make_item("Test Song", "Artist X", 240);
        item.tags = vec!["rock".to_string(), "indie".to_string()];
        item.play_count = 5;
        item.rating = Some(4);
        let text = format_item_for_rag(&item);
        assert!(text.contains("Test Song"));
        assert!(text.contains("Artist X"));
        assert!(text.contains("4:00"));
        assert!(text.contains("rock"));
        assert!(text.contains("Play count: 5"));
        assert!(text.contains("Rating: 4/5"));
    }

    #[test]
    fn format_item_rag_minimal() {
        let item = make_item("Minimal", "Nobody", 60);
        let text = format_item_for_rag(&item);
        assert!(text.contains("Minimal"));
        assert!(text.contains("1:00"));
        assert!(!text.contains("Tags:"));
    }

    #[test]
    fn build_library_context_formats() {
        let mut lib = Library::new();
        let mut item = make_item("Song A", "Band X", 180);
        item.tags = vec!["jazz".to_string()];
        lib.add_item(item);
        lib.add_item(make_item("Song B", "Band Y", 240));

        let ctx = build_library_context(&lib);
        assert!(ctx.contains("Song A — Band X"));
        assert!(ctx.contains("[jazz]"));
        assert!(ctx.contains("Song B — Band Y"));
    }

    #[test]
    fn build_library_context_caps_at_100() {
        let mut lib = Library::new();
        for i in 0..150 {
            lib.add_item(make_item(&format!("Song {i}"), "Artist", 120));
        }
        let ctx = build_library_context(&lib);
        let lines: Vec<&str> = ctx.lines().collect();
        assert_eq!(lines.len(), 100);
    }

    #[test]
    fn parse_valid_json_recommendation() {
        let json = r#"{"suggestions":[{"title":"Song A","artist":"Band","reason":"matches mood"}],"mood":"chill"}"#;
        let rec = parse_llm_recommendation(json).unwrap();
        assert_eq!(rec.suggestions.len(), 1);
        assert_eq!(rec.suggestions[0].title, "Song A");
        assert_eq!(rec.mood, Some("chill".to_string()));
    }

    #[test]
    fn parse_json_in_markdown() {
        let response = "Here are my picks:\n```json\n{\"suggestions\":[{\"title\":\"X\",\"artist\":null,\"reason\":\"great\"}],\"mood\":\"upbeat\"}\n```";
        let rec = parse_llm_recommendation(response).unwrap();
        assert_eq!(rec.suggestions[0].title, "X");
    }

    #[test]
    fn parse_fallback_raw_text() {
        let response = "I couldn't find any matching songs.";
        let rec = parse_llm_recommendation(response).unwrap();
        assert_eq!(rec.suggestions.len(), 1);
        assert!(rec.suggestions[0].title.contains("couldn't find"));
    }

    #[test]
    fn similar_media_serialization() {
        let sm = SimilarMedia {
            path: "/music/test.flac".to_string(),
            score: 0.92,
            metadata: serde_json::json!({"artist": "Test"}),
        };
        let json = serde_json::to_string(&sm).unwrap();
        let parsed: SimilarMedia = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "/music/test.flac");
    }

    #[test]
    fn llm_recommendation_serialization() {
        let rec = LlmRecommendation {
            suggestions: vec![LlmSuggestion {
                title: "Song".to_string(),
                artist: Some("Artist".to_string()),
                reason: "good".to_string(),
            }],
            mood: Some("chill".to_string()),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let parsed: LlmRecommendation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.suggestions.len(), 1);
    }

    #[test]
    fn transcription_result_serialization() {
        let tr = TranscriptionResult {
            text: "hello world".to_string(),
            language: "en".to_string(),
            segments: vec![TranscriptionSegment {
                start: 0.0,
                end: 1.5,
                text: "hello world".to_string(),
            }],
        };
        let json = serde_json::to_string(&tr).unwrap();
        let parsed: TranscriptionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.text, "hello world");
    }
}
