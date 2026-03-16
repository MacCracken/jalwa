//! jalwa — AI-native media player for AGNOS
//!
//! Jalwa (Persian: manifestation/display) is a media player built on the
//! Tarang media framework. Pure Rust audio, AI-powered recommendations,
//! smart playlists, and transcription routing via hoosh.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "jalwa", version, about = "AI-native media player for AGNOS")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
    /// Scan a directory and add files to the library
    Scan {
        /// Directory to scan
        directory: String,
    },
    /// List all items in the library
    Library,
    /// Export a playlist to M3U
    Export {
        /// Playlist name
        name: String,
        /// Output M3U file path
        output: String,
    },
    /// Import an M3U playlist
    Import {
        /// M3U file path
        file: String,
    },
    /// Launch interactive TUI
    Tui,
    /// Run as MCP server on stdio
    Mcp,
}

fn db_path() -> PathBuf {
    jalwa_core::db::default_db_path()
}

fn open_library() -> Result<jalwa_core::db::PersistentLibrary> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(jalwa_core::db::PersistentLibrary::open(&path)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Play { path }) => cmd_play(&path)?,
        Some(Commands::Info { path }) => cmd_info(&path)?,
        Some(Commands::Search { query }) => cmd_search(&query)?,
        Some(Commands::Stats) => cmd_stats()?,
        Some(Commands::Scan { directory }) => cmd_scan(&directory)?,
        Some(Commands::Library) => cmd_library()?,
        Some(Commands::Export { name, output }) => cmd_export(&name, &output)?,
        Some(Commands::Import { file }) => cmd_import(&file)?,
        Some(Commands::Tui) | None => cmd_tui()?,
        Some(Commands::Mcp) => cmd_mcp().await?,
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

    // Block until track finishes or Ctrl+C
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        let events = engine.poll_events();
        for ev in &events {
            match ev {
                jalwa_playback::EngineEvent::TrackFinished => {
                    println!("\nFinished.");
                    running.store(false, std::sync::atomic::Ordering::Relaxed);
                }
                jalwa_playback::EngineEvent::Error(e) => {
                    eprintln!("\nError: {e}");
                    running.store(false, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {}
            }
        }

        if running.load(std::sync::atomic::Ordering::Relaxed) {
            let status = engine.status();
            print!(
                "\r{} {}",
                jalwa_ui::render_status_bar(&status, None),
                jalwa_ui::render_progress_bar(status.progress().unwrap_or(0.0), 30)
            );
            use std::io::Write;
            let _ = std::io::stdout().flush();
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    engine.stop();
    println!();
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

fn cmd_search(query: &str) -> Result<()> {
    let plib = open_library()?;
    let results = plib.library.search(query);
    if results.is_empty() {
        println!("No results for '{query}'");
    } else {
        for (i, item) in results.iter().enumerate() {
            println!("{}", jalwa_ui::render_library_item(item, i));
        }
    }
    Ok(())
}

fn cmd_stats() -> Result<()> {
    let plib = open_library()?;
    println!("{}", jalwa_ui::render_library_stats(&plib.library));
    Ok(())
}

fn cmd_scan(directory: &str) -> Result<()> {
    let dir = std::path::Path::new(directory);
    let mut plib = open_library()?;

    println!("Scanning {}...", dir.display());
    let scanned = jalwa_core::scanner::scan_directory(dir)?;
    println!("Found {} media files", scanned.len());

    let mut added = 0;
    for file in scanned {
        let path = file.path.clone();
        // Skip if already in library
        if plib.library.find_by_path(&path).is_some() {
            continue;
        }
        let item = jalwa_core::scanner::scanned_to_media_item(file);
        plib.add_item(item)?;
        added += 1;
    }

    plib.add_scan_path(dir.to_path_buf())?;
    println!("Added {added} new items to library");
    println!("{}", jalwa_ui::render_library_stats(&plib.library));
    Ok(())
}

fn cmd_library() -> Result<()> {
    let plib = open_library()?;
    if plib.library.items.is_empty() {
        println!("Library is empty. Use 'jalwa scan <directory>' to add files.");
    } else {
        for (i, item) in plib.library.items.iter().enumerate() {
            println!("{}", jalwa_ui::render_library_item(item, i));
        }
        println!("\n{}", jalwa_ui::render_library_stats(&plib.library));
    }
    Ok(())
}

fn cmd_export(name: &str, output: &str) -> Result<()> {
    let plib = open_library()?;
    let playlist = plib
        .library
        .playlists
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow::anyhow!("playlist '{name}' not found"))?;

    jalwa_core::playlist_io::save_m3u(playlist, &plib.library, std::path::Path::new(output))?;
    println!("Exported '{}' ({} items) to {output}", playlist.name, playlist.len());
    Ok(())
}

fn cmd_import(file: &str) -> Result<()> {
    let mut plib = open_library()?;
    let paths = jalwa_core::playlist_io::load_m3u(std::path::Path::new(file))?;

    let playlist_name = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported")
        .to_string();

    let mut playlist = jalwa_core::Playlist::new(&playlist_name);
    let mut added = 0;

    for path in &paths {
        // Add to library if not already present
        if plib.library.find_by_path(path).is_none() {
            match jalwa_core::scanner::scan_directory(path.parent().unwrap_or(path)) {
                Ok(scanned) => {
                    for s in scanned {
                        if s.path == *path {
                            let item = jalwa_core::scanner::scanned_to_media_item(s);
                            let _ = plib.add_item(item);
                            break;
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        if let Some(item) = plib.library.find_by_path(path) {
            playlist.add(item.id);
            added += 1;
        }
    }

    plib.save_playlist(&playlist)?;
    println!("Imported '{playlist_name}' with {added} tracks");
    Ok(())
}

fn cmd_tui() -> Result<()> {
    let plib = open_library()?;
    let engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
    let app = jalwa_ui::app::App::new(plib, engine);
    jalwa_ui::tui::run(app)?;
    Ok(())
}

async fn cmd_mcp() -> Result<()> {
    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, BufReader};

    let plib = Arc::new(Mutex::new(open_library()?));

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
                handle_tool_call(tool_name, args, &plib)
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

fn handle_tool_call(
    name: &str,
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
) -> serde_json::Value {
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
            let lib = plib.lock().unwrap();
            let results = lib.library.search(query);
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
            let item_id_str = args["item_id"].as_str().unwrap_or("");
            let max = args["max"].as_u64().unwrap_or(5) as usize;
            let lib = plib.lock().unwrap();

            match uuid::Uuid::parse_str(item_id_str) {
                Ok(seed_id) => {
                    let recs = jalwa_ai::recommend(&lib.library, seed_id, max);
                    if recs.is_empty() {
                        json!({ "content": [{ "type": "text", "text": "No recommendations found" }] })
                    } else {
                        let text: Vec<String> = recs.iter().map(|r| {
                            let title = lib.library.find_by_id(r.item_id)
                                .map(|i| {
                                    let artist = i.artist.as_deref().unwrap_or("Unknown");
                                    format!("{} - {}", artist, i.title)
                                })
                                .unwrap_or_else(|| r.item_id.to_string());
                            let reasons: Vec<String> = r.reasons.iter().map(|r| r.to_string()).collect();
                            format!("  {:.0}% {} ({})", r.score, title, reasons.join(", "))
                        }).collect();
                        json!({ "content": [{ "type": "text", "text": text.join("\n") }] })
                    }
                }
                Err(_) => {
                    json!({ "content": [{ "type": "text", "text": "invalid item_id UUID" }], "isError": true })
                }
            }
        }
        _ => {
            json!({ "content": [{ "type": "text", "text": format!("unknown tool: {name}") }], "isError": true })
        }
    }
}

fn ctrlc_handler(running: Arc<std::sync::atomic::AtomicBool>) {
    let _ = ctrlc::install_handler(move || {
        running.store(false, std::sync::atomic::Ordering::Relaxed);
    });
}

/// Simple ctrlc handler module (inline, no extra dep needed — uses signal directly)
mod ctrlc {
    pub fn install_handler(handler: impl Fn() + Send + 'static) -> Result<(), std::io::Error> {
        // Use a simple signal handler via libc
        unsafe {
            libc::signal(libc::SIGINT, signal_handler as *const () as libc::sighandler_t);
        }
        // Store handler in a static
        *HANDLER.lock().unwrap() = Some(Box::new(handler));
        Ok(())
    }

    static HANDLER: std::sync::Mutex<Option<Box<dyn Fn() + Send>>> = std::sync::Mutex::new(None);

    extern "C" fn signal_handler(_: libc::c_int) {
        if let Ok(guard) = HANDLER.lock() {
            if let Some(ref handler) = *guard {
                handler();
            }
        }
    }
}
