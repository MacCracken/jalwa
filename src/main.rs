//! jalwa — AI-native media player for AGNOS
//!
//! Jalwa (Persian: manifestation/display) is a media player built on the
//! Tarang media framework. Pure Rust audio, AI-powered recommendations,
//! smart playlists, and transcription routing via hoosh.

mod mcp;

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
    /// Launch desktop GUI
    Gui,
    /// Run as MCP server on stdio
    Mcp,
}

fn db_path() -> PathBuf {
    jalwa_core::db::default_db_path()
}

fn open_library() -> Result<jalwa_core::db::PersistentLibrary> {
    open_library_at(&db_path())
}

fn open_library_at(path: &std::path::Path) -> Result<jalwa_core::db::PersistentLibrary> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(jalwa_core::db::PersistentLibrary::open(path)?)
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
        Some(Commands::Tui) => cmd_tui()?,
        Some(Commands::Gui) | None => cmd_gui()?,
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
    let info = tarang::audio::probe_audio(file)?;

    println!("File:     {path}");
    println!("Format:   {}", info.format);
    if let Some(d) = info.duration {
        println!("Duration: {}", jalwa_playback::format_duration(d));
    }
    println!("Streams:  {}", info.streams.len());

    for (i, stream) in info.streams.iter().enumerate() {
        match stream {
            tarang::core::StreamInfo::Audio(a) => {
                println!(
                    "  [{}] Audio: {} {}Hz {}ch",
                    i, a.codec, a.sample_rate, a.channels
                );
            }
            tarang::core::StreamInfo::Video(v) => {
                println!(
                    "  [{}] Video: {} {}x{} {:.1}fps",
                    i, v.codec, v.width, v.height, v.frame_rate
                );
            }
            tarang::core::StreamInfo::Subtitle { language } => {
                println!(
                    "  [{}] Subtitle: {}",
                    i,
                    language.as_deref().unwrap_or("unknown")
                );
            }
        }
    }

    // AI analysis
    let analysis = tarang::ai::analyze_media(&info);
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
    println!(
        "Exported '{}' ({} items) to {output}",
        playlist.name,
        playlist.len()
    );
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

fn cmd_gui() -> Result<()> {
    let plib = open_library()?;
    let engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
    jalwa_gui::run(plib, engine).map_err(|e| anyhow::anyhow!("GUI error: {e}"))
}

fn cmd_tui() -> Result<()> {
    let plib = open_library()?;
    let engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
    let app = jalwa_ui::app::App::new(plib, engine);
    jalwa_ui::tui::run(app)?;
    Ok(())
}

async fn cmd_mcp() -> Result<()> {
    let plib = Arc::new(Mutex::new(open_library()?));
    let engine = Arc::new(Mutex::new(jalwa_playback::PlaybackEngine::new(
        jalwa_playback::EngineConfig::default(),
    )));
    mcp::run(plib, engine).await
}

fn ctrlc_handler(running: Arc<std::sync::atomic::AtomicBool>) {
    let _ = ctrlc::install_handler(move || {
        running.store(false, std::sync::atomic::Ordering::Relaxed);
    });
}

/// Minimal WAV generator for tests — delegates to shared fixture.
#[cfg(test)]
fn make_test_wav(num_samples: u32, sample_rate: u32) -> Vec<u8> {
    jalwa_core::test_fixtures::make_test_wav(num_samples, sample_rate)
}

/// Simple ctrlc handler module (inline, no extra dep needed — uses signal directly)
mod ctrlc {
    pub fn install_handler(handler: impl Fn() + Send + 'static) -> Result<(), std::io::Error> {
        // Use a simple signal handler via libc
        unsafe {
            libc::signal(
                libc::SIGINT,
                signal_handler as *const () as libc::sighandler_t,
            );
        }
        // Store handler in a static
        *HANDLER.lock().unwrap() = Some(Box::new(handler));
        Ok(())
    }

    static HANDLER: std::sync::Mutex<Option<Box<dyn Fn() + Send>>> = std::sync::Mutex::new(None);

    extern "C" fn signal_handler(_: libc::c_int) {
        if let Ok(guard) = HANDLER.lock()
            && let Some(ref handler) = *guard
        {
            handler();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_db() -> (PathBuf, jalwa_core::db::PersistentLibrary) {
        let path = std::env::temp_dir().join(format!("jalwa_cmd_test_{}.db", uuid::Uuid::new_v4()));
        let plib = open_library_at(&path).unwrap();
        (path, plib)
    }

    fn tmp_dir_with_wav() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jalwa_cmd_wav_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = make_test_wav(4410, 44100);
        std::fs::write(dir.join("test.wav"), &wav).unwrap();
        dir
    }

    #[test]
    fn info_with_wav() {
        let dir = tmp_dir_with_wav();
        let wav_path = dir.join("test.wav");
        let result = cmd_info(wav_path.to_str().unwrap());
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn info_nonexistent() {
        let result = cmd_info("/nonexistent/file.wav");
        assert!(result.is_err());
    }

    #[test]
    fn search_empty_library() {
        let (path, _plib) = tmp_db();
        let plib = open_library_at(&path).unwrap();
        let results = plib.library.search("anything");
        assert!(results.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn stats_empty_library() {
        let (path, plib) = tmp_db();
        let stats = jalwa_ui::render_library_stats(&plib.library);
        assert!(stats.contains("0 items"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn library_empty() {
        let (path, plib) = tmp_db();
        assert!(plib.library.items.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn scan_with_wav() {
        let dir = tmp_dir_with_wav();
        let (db_path, mut plib) = tmp_db();

        let scanned = jalwa_core::scanner::scan_directory(&dir).unwrap();
        assert_eq!(scanned.len(), 1);

        for file in scanned {
            let item = jalwa_core::scanner::scanned_to_media_item(file);
            plib.add_item(item).unwrap();
        }
        assert_eq!(plib.library.items.len(), 1);
        assert_eq!(plib.library.items[0].title, "test");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn scan_nonexistent_dir() {
        let result = jalwa_core::scanner::scan_directory(std::path::Path::new("/nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn export_missing_playlist() {
        let (path, plib) = tmp_db();
        let found = plib.library.playlists.iter().find(|p| p.name == "nope");
        assert!(found.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_library_at_creates_dir() {
        let dir = std::env::temp_dir().join(format!("jalwa_deep_{}/sub/dir", uuid::Uuid::new_v4()));
        let db = dir.join("test.db");
        let result = open_library_at(&db);
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn search_with_results() {
        let (path, mut plib) = tmp_db();
        plib.library
            .add_item(jalwa_core::test_fixtures::make_media_item(
                "Bohemian Rhapsody",
                "Queen",
                354,
            ));
        let results = plib.library.search("queen");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist.as_deref(), Some("Queen"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn scan_skips_duplicates() {
        let dir = tmp_dir_with_wav();
        let (db_path, mut plib) = tmp_db();

        let scanned = jalwa_core::scanner::scan_directory(&dir).unwrap();
        for file in scanned {
            let item = jalwa_core::scanner::scanned_to_media_item(file);
            plib.add_item(item).unwrap();
        }
        assert_eq!(plib.library.items.len(), 1);

        // Second scan — should skip duplicates
        let scanned2 = jalwa_core::scanner::scan_directory(&dir).unwrap();
        let mut added = 0;
        for file in scanned2 {
            let p = file.path.clone();
            if plib.library.find_by_path(&p).is_some() {
                continue;
            }
            let item = jalwa_core::scanner::scanned_to_media_item(file);
            plib.add_item(item).unwrap();
            added += 1;
        }
        assert_eq!(added, 0);
        assert_eq!(plib.library.items.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn export_import_roundtrip() {
        let dir = tmp_dir_with_wav();
        let (db_path, mut plib) = tmp_db();

        let scanned = jalwa_core::scanner::scan_directory(&dir).unwrap();
        for file in scanned {
            let item = jalwa_core::scanner::scanned_to_media_item(file);
            plib.add_item(item).unwrap();
        }

        let item_id = plib.library.items[0].id;
        let mut playlist = jalwa_core::Playlist::new("Test PL");
        playlist.add(item_id);
        plib.save_playlist(&playlist).unwrap();

        let m3u_path =
            std::env::temp_dir().join(format!("jalwa_export_{}.m3u", uuid::Uuid::new_v4()));
        let export_result = jalwa_core::playlist_io::save_m3u(&playlist, &plib.library, &m3u_path);
        assert!(export_result.is_ok());
        assert!(m3u_path.exists());

        let paths = jalwa_core::playlist_io::load_m3u(&m3u_path).unwrap();
        assert_eq!(paths.len(), 1);

        let _ = std::fs::remove_file(&m3u_path);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn info_shows_streams() {
        let dir = tmp_dir_with_wav();
        let wav_path = dir.join("test.wav");
        let result = cmd_info(wav_path.to_str().unwrap());
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stats_with_items() {
        let (path, mut plib) = tmp_db();
        plib.library
            .add_item(jalwa_core::test_fixtures::make_media_item("A", "X", 100));
        plib.library
            .add_item(jalwa_core::test_fixtures::make_media_item("B", "Y", 200));
        let stats = jalwa_ui::render_library_stats(&plib.library);
        assert!(stats.contains("2 items"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn library_with_items() {
        let (path, mut plib) = tmp_db();
        plib.library
            .add_item(jalwa_core::test_fixtures::make_media_item(
                "Song", "Band", 180,
            ));
        assert_eq!(plib.library.items.len(), 1);
        let rendered = jalwa_ui::render_library_item(&plib.library.items[0], 0);
        assert!(rendered.contains("Band"));
        assert!(rendered.contains("Song"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn scan_adds_path() {
        let dir = tmp_dir_with_wav();
        let (db_path, mut plib) = tmp_db();
        plib.add_scan_path(dir.to_path_buf()).unwrap();
        assert_eq!(plib.library.scan_paths.len(), 1);
        assert_eq!(plib.library.scan_paths[0], dir);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn db_path_is_valid() {
        let p = db_path();
        assert!(p.to_str().unwrap().contains("jalwa"));
    }

    #[test]
    fn import_finds_existing_items() {
        let dir = tmp_dir_with_wav();
        let (db_path, mut plib) = tmp_db();

        let scanned = jalwa_core::scanner::scan_directory(&dir).unwrap();
        for file in scanned {
            let item = jalwa_core::scanner::scanned_to_media_item(file);
            plib.add_item(item).unwrap();
        }
        assert_eq!(plib.library.items.len(), 1);

        let wav_path = dir.join("test.wav");
        assert!(plib.library.find_by_path(&wav_path).is_some());

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&db_path);
    }
}
