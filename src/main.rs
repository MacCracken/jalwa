//! jalwa — AI-native media player for AGNOS
//!
//! Jalwa (Persian: manifestation/display) is a media player built on the
//! Tarang media framework. Pure Rust audio, AI-powered recommendations,
//! smart playlists, and transcription routing via hoosh.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jalwa", version, about = "AI-native media player for AGNOS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Play a media file
    Play {
        /// Path to media file
        path: String,
    },
    /// Probe a media file and display info
    Info {
        /// Path to media file
        path: String,
    },
    /// Search the library
    Search {
        /// Search query
        query: String,
    },
    /// Show library statistics
    Stats,
    /// Run as MCP server on stdio
    Mcp,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Play { path } => cmd_play(&path)?,
        Commands::Info { path } => cmd_info(&path)?,
        Commands::Search { query } => cmd_search(&query),
        Commands::Stats => cmd_stats(),
        Commands::Mcp => cmd_mcp().await?,
    }

    Ok(())
}

fn cmd_play(path: &str) -> Result<()> {
    let mut engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());

    let p = std::path::Path::new(path);
    engine.open(p)?;

    println!(
        "Loaded: {} ({})",
        p.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default(),
        engine
            .duration()
            .map(jalwa_playback::format_duration)
            .unwrap_or_else(|| "unknown duration".to_string())
    );

    engine.play()?;
    let status = engine.status();
    println!("{}", jalwa_ui::render_status_bar(&status, None));
    println!(
        "{}",
        jalwa_ui::render_progress_bar(status.progress().unwrap_or(0.0), 40)
    );

    // TODO: actual audio output loop via PipeWire
    println!("(PipeWire output not yet implemented — tarang decode ready)");

    Ok(())
}

fn cmd_info(path: &str) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let info = tarang_audio::probe_audio(file)?;

    println!("File:     {path}");
    println!("Format:   {}", info.format);
    if let Some(d) = info.duration {
        println!("Duration: {}", jalwa_playback::format_duration(d));
    }
    println!("Streams:  {}", info.streams.len());

    for (i, stream) in info.streams.iter().enumerate() {
        match stream {
            tarang_core::StreamInfo::Audio(a) => {
                println!(
                    "  [{}] Audio: {} {}Hz {}ch",
                    i, a.codec, a.sample_rate, a.channels
                );
            }
            tarang_core::StreamInfo::Video(v) => {
                println!(
                    "  [{}] Video: {} {}x{} {:.1}fps",
                    i, v.codec, v.width, v.height, v.frame_rate
                );
            }
            tarang_core::StreamInfo::Subtitle { language } => {
                println!(
                    "  [{}] Subtitle: {}",
                    i,
                    language.as_deref().unwrap_or("unknown")
                );
            }
        }
    }

    // AI analysis
    let analysis = tarang_ai::analyze_media(&info);
    println!("\nAI Analysis:");
    println!("  Type:       {}", analysis.content_type);
    println!("  Quality:    {:.0}/100", analysis.quality_score);
    println!("  Tags:       {}", analysis.tags.join(", "));
    if let Some(rec) = analysis.codec_recommendation {
        println!("  Suggestion: {rec}");
    }

    Ok(())
}

fn cmd_search(query: &str) {
    let lib = jalwa_core::Library::new();
    let results = lib.search(query);
    if results.is_empty() {
        println!("No results for '{query}' (library is empty — scan a directory first)");
    } else {
        for (i, item) in results.iter().enumerate() {
            println!("{}", jalwa_ui::render_library_item(item, i));
        }
    }
}

fn cmd_stats() {
    let lib = jalwa_core::Library::new();
    println!("{}", jalwa_ui::render_library_stats(&lib));
}

async fn cmd_mcp() -> Result<()> {
    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = request["method"].as_str().unwrap_or("");
        let id = &request["id"];

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "jalwa", "version": env!("CARGO_PKG_VERSION") }
            }),
            "tools/list" => json!({
                "tools": [
                    {
                        "name": "jalwa_play",
                        "description": "Play a media file (audio or video)",
                        "inputSchema": {
                            "type": "object",
                            "properties": { "path": { "type": "string" } },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "jalwa_pause",
                        "description": "Pause current playback",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "jalwa_status",
                        "description": "Get current playback status (state, position, volume)",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "jalwa_search",
                        "description": "Search the media library by title, artist, album, or tag",
                        "inputSchema": {
                            "type": "object",
                            "properties": { "query": { "type": "string" } },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "jalwa_recommend",
                        "description": "Get AI-powered media recommendations based on a seed item",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "item_id": { "type": "string", "description": "UUID of seed media item" },
                                "max": { "type": "integer", "description": "Max recommendations (default 5)" }
                            },
                            "required": ["item_id"]
                        }
                    }
                ]
            }),
            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                let args = &request["params"]["arguments"];
                handle_tool_call(tool_name, args)
            }
            _ => json!({ "error": format!("unknown method: {method}") }),
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        println!("{}", serde_json::to_string(&response)?);
    }

    Ok(())
}

fn handle_tool_call(name: &str, args: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;

    match name {
        "jalwa_play" => {
            let path = args["path"].as_str().unwrap_or("");
            let mut engine =
                jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
            match engine.open(std::path::Path::new(path)) {
                Ok(()) => {
                    let _ = engine.play();
                    let status = engine.status();
                    json!({
                        "content": [{ "type": "text", "text": format!("Playing: {path}\n{}", jalwa_ui::render_status_bar(&status, None)) }]
                    })
                }
                Err(e) => {
                    json!({ "content": [{ "type": "text", "text": format!("error: {e}") }], "isError": true })
                }
            }
        }
        "jalwa_pause" => {
            json!({ "content": [{ "type": "text", "text": "paused" }] })
        }
        "jalwa_status" => {
            let engine =
                jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
            let status = engine.status();
            json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&status).unwrap_or_default() }]
            })
        }
        "jalwa_search" => {
            let query = args["query"].as_str().unwrap_or("");
            let lib = jalwa_core::Library::new();
            let results = lib.search(query);
            let text = if results.is_empty() {
                format!("No results for '{query}'")
            } else {
                results
                    .iter()
                    .enumerate()
                    .map(|(i, item)| jalwa_ui::render_library_item(item, i))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            json!({ "content": [{ "type": "text", "text": text }] })
        }
        "jalwa_recommend" => {
            json!({ "content": [{ "type": "text", "text": "recommendations require a populated library" }] })
        }
        _ => {
            json!({ "content": [{ "type": "text", "text": format!("unknown tool: {name}") }], "isError": true })
        }
    }
}
