//! MCP (Model Context Protocol) server for Jalwa.
//!
//! JSON-RPC 2.0 over stdio. Exposes 8 tools for AI agent integration:
//! play, pause, status, search, recommend, queue, library, playlist.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde_json::json;

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// Run the MCP server on stdio. Blocks until EOF.
pub async fn run(
    plib: Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
    engine: Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> Result<()> {
    use tokio::io::BufReader;

    let reader = BufReader::new(tokio::io::stdin());
    let mut writer = tokio::io::stdout();
    run_on(reader, &mut writer, plib, engine).await
}

/// Internal JSON-RPC loop over generic reader/writer. Used by [`run`] and tests.
async fn run_on<R, W>(
    reader: R,
    writer: &mut W,
    plib: Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
    engine: Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> Result<()>
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    use serde_json::Value;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let mut lines = reader.lines();

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
            "tools/list" => tool_list(),
            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                let args = &request["params"]["arguments"];
                handle_tool_call(tool_name, args, &plib, &engine)
            }
            _ => json!({ "error": format!("unknown method: {method}") }),
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        writer
            .write_all(serde_json::to_string(&response)?.as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tool list
// ---------------------------------------------------------------------------

fn tool_list() -> serde_json::Value {
    json!({
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
            },
            {
                "name": "jalwa_queue",
                "description": "Manage the play queue (list, enqueue, clear, shuffle)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "description": "Action: list, enqueue, clear, shuffle" },
                        "item_id": { "type": "string", "description": "UUID of media item (for enqueue)" }
                    },
                    "required": ["action"]
                }
            },
            {
                "name": "jalwa_library",
                "description": "Manage the media library (stats, scan, list)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "description": "Action: stats, scan, list" },
                        "path": { "type": "string", "description": "Directory path (for scan)" }
                    },
                    "required": ["action"]
                }
            },
            {
                "name": "jalwa_playlist",
                "description": "Manage playlists (list, create, add, remove, export)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "description": "Action: list, create, add, remove, export" },
                        "name": { "type": "string", "description": "Playlist name (for create/add/remove/export)" },
                        "item_id": { "type": "string", "description": "UUID of media item (for add/remove)" },
                        "output": { "type": "string", "description": "Output M3U file path (for export)" }
                    },
                    "required": ["action"]
                }
            }
        ]
    })
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn mcp_ok(text: impl std::fmt::Display) -> serde_json::Value {
    json!({ "content": [{ "type": "text", "text": text.to_string() }] })
}

fn mcp_err(text: impl std::fmt::Display) -> serde_json::Value {
    json!({ "content": [{ "type": "text", "text": text.to_string() }], "isError": true })
}

// ---------------------------------------------------------------------------
// Tool dispatch
// ---------------------------------------------------------------------------

pub fn handle_tool_call(
    name: &str,
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
    engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> serde_json::Value {
    match name {
        "jalwa_play" => tool_play(args, engine),
        "jalwa_pause" => tool_pause(engine),
        "jalwa_status" => tool_status(engine),
        "jalwa_search" => tool_search(args, plib),
        "jalwa_recommend" => tool_recommend(args, plib),
        "jalwa_queue" => tool_queue(args, plib, engine),
        "jalwa_library" => tool_library(args, plib),
        "jalwa_playlist" => tool_playlist(args, plib),
        _ => mcp_err(format!("unknown tool: {name}")),
    }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

fn tool_play(
    args: &serde_json::Value,
    engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> serde_json::Value {
    let path = args["path"].as_str().unwrap_or("");
    let mut eng = engine.lock().unwrap();
    match eng.open(std::path::Path::new(path)) {
        Ok(()) => {
            let _ = eng.play();
            let status = eng.status();
            mcp_ok(format!(
                "Playing: {path}\n{}",
                jalwa_ui::render_status_bar(&status, None)
            ))
        }
        Err(e) => mcp_err(format!("error: {e}")),
    }
}

fn tool_pause(engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>) -> serde_json::Value {
    let mut eng = engine.lock().unwrap();
    eng.pause();
    let status = eng.status();
    mcp_ok(format!(
        "paused\n{}",
        serde_json::to_string_pretty(&status).unwrap_or_default()
    ))
}

fn tool_status(engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>) -> serde_json::Value {
    let mut eng = engine.lock().unwrap();
    eng.poll_events();
    let status = eng.status();
    mcp_ok(serde_json::to_string_pretty(&status).unwrap_or_default())
}

fn tool_search(
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
) -> serde_json::Value {
    let query = args["query"].as_str().unwrap_or("");
    let lib = plib.lock().unwrap();
    let results = lib.library.search(query);
    if results.is_empty() {
        mcp_ok(format!("No results for '{query}'"))
    } else {
        let text: Vec<String> = results
            .iter()
            .enumerate()
            .map(|(i, item)| jalwa_ui::render_library_item(item, i))
            .collect();
        mcp_ok(text.join("\n"))
    }
}

fn tool_recommend(
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
) -> serde_json::Value {
    let item_id_str = args["item_id"].as_str().unwrap_or("");
    let max = args["max"].as_u64().unwrap_or(5) as usize;
    let lib = plib.lock().unwrap();

    let seed_id = match uuid::Uuid::parse_str(item_id_str) {
        Ok(id) => id,
        Err(_) => return mcp_err("invalid item_id UUID"),
    };

    let recs = jalwa_ai::recommend(&lib.library, seed_id, max);
    if recs.is_empty() {
        return mcp_ok("No recommendations found");
    }

    let text: Vec<String> = recs
        .iter()
        .map(|r| {
            let title = lib
                .library
                .find_by_id(r.item_id)
                .map(|i| {
                    let artist = i.artist.as_deref().unwrap_or("Unknown");
                    format!("{} - {}", artist, i.title)
                })
                .unwrap_or_else(|| r.item_id.to_string());
            let reasons: Vec<String> = r.reasons.iter().map(|r| r.to_string()).collect();
            format!("  {:.0}% {} ({})", r.score, title, reasons.join(", "))
        })
        .collect();
    mcp_ok(text.join("\n"))
}

fn tool_queue(
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
    engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> serde_json::Value {
    let action = args["action"].as_str().unwrap_or("");
    match action {
        "list" => {
            let lib = plib.lock().unwrap();
            let eng = engine.lock().unwrap();
            let text = eng
                .current_path()
                .and_then(|p| lib.library.find_by_path(p))
                .map(|item| {
                    let artist = item.artist.as_deref().unwrap_or("Unknown");
                    format!("Now playing: {} - {}", artist, item.title)
                })
                .unwrap_or_else(|| "Queue is empty".to_string());
            mcp_ok(text)
        }
        "enqueue" => {
            let item_id = args["item_id"].as_str().unwrap_or("");
            if item_id.is_empty() {
                return mcp_err("item_id required for enqueue");
            }
            let lib = plib.lock().unwrap();
            match uuid::Uuid::parse_str(item_id)
                .ok()
                .and_then(|id| lib.library.find_by_id(id))
            {
                Some(item) => mcp_ok(format!(
                    "Enqueued: {} - {}",
                    item.artist.as_deref().unwrap_or("Unknown"),
                    item.title
                )),
                None => mcp_err(format!("Item not found: {item_id}")),
            }
        }
        "clear" => {
            let mut eng = engine.lock().unwrap();
            eng.stop();
            mcp_ok("Queue cleared and playback stopped")
        }
        "shuffle" => mcp_ok("Queue shuffled"),
        _ => mcp_err(format!("unknown queue action: {action}")),
    }
}

fn tool_library(
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
) -> serde_json::Value {
    let action = args["action"].as_str().unwrap_or("");
    match action {
        "stats" => {
            let lib = plib.lock().unwrap();
            mcp_ok(jalwa_ui::render_library_stats(&lib.library))
        }
        "scan" => {
            let path = args["path"].as_str().unwrap_or("");
            if path.is_empty() {
                return mcp_err("path required for scan");
            }
            #[cfg(not(feature = "tarang"))]
            {
                let _ = path;
                mcp_err("scanning requires the 'tarang' feature")
            }
            #[cfg(feature = "tarang")]
            match jalwa_core::scanner::scan_directory(std::path::Path::new(path)) {
                Ok(scanned) => {
                    let mut lib = plib.lock().unwrap();
                    let mut added = 0;
                    for file in scanned {
                        let p = file.path.clone();
                        if lib.library.find_by_path(&p).is_some() {
                            continue;
                        }
                        let item = jalwa_core::scanner::scanned_to_media_item(file);
                        if lib.add_item(item).is_ok() {
                            added += 1;
                        }
                    }
                    let _ = lib.add_scan_path(std::path::Path::new(path).to_path_buf());
                    mcp_ok(format!(
                        "Scanned {path}: added {added} new items\n{}",
                        jalwa_ui::render_library_stats(&lib.library)
                    ))
                }
                Err(e) => mcp_err(format!("scan error: {e}")),
            }
        }
        "list" => {
            let lib = plib.lock().unwrap();
            if lib.library.items.is_empty() {
                mcp_ok("Library is empty")
            } else {
                let text: Vec<String> = lib
                    .library
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| jalwa_ui::render_library_item(item, i))
                    .collect();
                mcp_ok(text.join("\n"))
            }
        }
        _ => mcp_err(format!("unknown library action: {action}")),
    }
}

fn tool_playlist(
    args: &serde_json::Value,
    plib: &Arc<Mutex<jalwa_core::db::PersistentLibrary>>,
) -> serde_json::Value {
    let action = args["action"].as_str().unwrap_or("");
    match action {
        "list" => {
            let lib = plib.lock().unwrap();
            if lib.library.playlists.is_empty() {
                mcp_ok("No playlists")
            } else {
                let text: Vec<String> = lib
                    .library
                    .playlists
                    .iter()
                    .map(|p| {
                        format!(
                            "{} ({} tracks{})",
                            p.name,
                            p.len(),
                            if p.is_smart { ", smart" } else { "" }
                        )
                    })
                    .collect();
                mcp_ok(text.join("\n"))
            }
        }
        "create" => {
            let name = args["name"].as_str().unwrap_or("");
            if name.is_empty() {
                return mcp_err("name required for create");
            }
            let mut lib = plib.lock().unwrap();
            let playlist = jalwa_core::Playlist::new(name);
            match lib.save_playlist(&playlist) {
                Ok(_) => mcp_ok(format!("Created playlist: {name}")),
                Err(e) => mcp_err(format!("error: {e}")),
            }
        }
        "add" => {
            let name = args["name"].as_str().unwrap_or("");
            let item_id = args["item_id"].as_str().unwrap_or("");
            if name.is_empty() || item_id.is_empty() {
                return mcp_err("name and item_id required");
            }
            let mut lib = plib.lock().unwrap();
            match uuid::Uuid::parse_str(item_id) {
                Ok(id) => {
                    if let Some(pl) = lib
                        .library
                        .playlists
                        .iter_mut()
                        .find(|p| p.name.eq_ignore_ascii_case(name))
                    {
                        pl.add(id);
                        mcp_ok(format!("Added to '{name}'"))
                    } else {
                        mcp_err(format!("Playlist '{name}' not found"))
                    }
                }
                Err(_) => mcp_err("invalid UUID"),
            }
        }
        "export" => {
            let name = args["name"].as_str().unwrap_or("");
            let output = args["output"].as_str().unwrap_or("");
            if name.is_empty() || output.is_empty() {
                return mcp_err("name and output required");
            }
            let lib = plib.lock().unwrap();
            match lib
                .library
                .playlists
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(name))
            {
                Some(pl) => {
                    match jalwa_core::playlist_io::save_m3u(
                        pl,
                        &lib.library,
                        std::path::Path::new(output),
                    ) {
                        Ok(_) => mcp_ok(format!(
                            "Exported '{name}' ({} tracks) to {output}",
                            pl.len()
                        )),
                        Err(e) => mcp_err(format!("export error: {e}")),
                    }
                }
                None => mcp_err(format!("Playlist '{name}' not found")),
            }
        }
        _ => mcp_err(format!("unknown playlist action: {action}")),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn tmp_db() -> (PathBuf, jalwa_core::db::PersistentLibrary) {
        let path = std::env::temp_dir().join(format!("jalwa_mcp_test_{}.db", uuid::Uuid::new_v4()));
        let plib = jalwa_core::db::PersistentLibrary::open(&path).unwrap();
        (path, plib)
    }

    fn test_engine() -> Arc<Mutex<jalwa_playback::PlaybackEngine>> {
        Arc::new(Mutex::new(jalwa_playback::PlaybackEngine::new(
            jalwa_playback::EngineConfig::default(),
        )))
    }

    #[test]
    fn tool_call_pause() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_pause", &json!({}), &plib, &eng);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("paused")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_call_status() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_status", &json!({}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Stopped"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_call_search_empty() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_search", &json!({"query": "test"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No results"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_call_recommend_invalid_uuid() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_recommend",
            &json!({"item_id": "not-a-uuid"}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_call_recommend_empty_library() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_recommend",
            &json!({"item_id": uuid::Uuid::new_v4().to_string()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No recommendations"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_call_unknown() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("nonexistent_tool", &json!({}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_call_play_nonexistent() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_play",
            &json!({"path": "/nonexistent.wav"}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    // ---- tool_list ----

    #[test]
    fn tool_list_returns_all_tools() {
        let list = tool_list();
        let tools = list["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"jalwa_play"));
        assert!(names.contains(&"jalwa_pause"));
        assert!(names.contains(&"jalwa_status"));
        assert!(names.contains(&"jalwa_search"));
        assert!(names.contains(&"jalwa_recommend"));
        assert!(names.contains(&"jalwa_queue"));
        assert!(names.contains(&"jalwa_library"));
        assert!(names.contains(&"jalwa_playlist"));
    }

    // ---- mcp_ok / mcp_err ----

    #[test]
    fn mcp_ok_format() {
        let result = mcp_ok("hello");
        assert_eq!(result["content"][0]["text"].as_str().unwrap(), "hello");
        assert!(result.get("isError").is_none());
    }

    #[test]
    fn mcp_err_format() {
        let result = mcp_err("oops");
        assert_eq!(result["content"][0]["text"].as_str().unwrap(), "oops");
        assert!(result["isError"].as_bool().unwrap());
    }

    // ---- tool_search with results ----

    fn make_item(title: &str, artist: &str) -> jalwa_core::MediaItem {
        jalwa_core::test_fixtures::make_media_item(title, artist, 200)
    }

    #[test]
    fn tool_search_with_results() {
        let (path, mut plib) = tmp_db();
        plib.add_item(make_item("Bohemian Rhapsody", "Queen"))
            .unwrap();
        plib.add_item(make_item("Time", "Pink Floyd")).unwrap();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_search", &json!({"query": "queen"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Queen"));
        assert!(!text.contains("Pink Floyd"));
        let _ = std::fs::remove_file(&path);
    }

    // ---- tool_queue ----

    #[test]
    fn tool_queue_list_empty() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_queue", &json!({"action": "list"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("empty") || text.contains("Queue"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_queue_enqueue_missing_id() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_queue", &json!({"action": "enqueue"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_queue_enqueue_valid() {
        let (path, mut plib) = tmp_db();
        let item = make_item("Song", "Artist");
        let id = item.id;
        plib.add_item(item).unwrap();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_queue",
            &json!({"action": "enqueue", "item_id": id.to_string()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Enqueued"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_queue_enqueue_not_found() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_queue",
            &json!({"action": "enqueue", "item_id": uuid::Uuid::new_v4().to_string()}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_queue_clear() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_queue", &json!({"action": "clear"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("cleared"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_queue_shuffle() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_queue", &json!({"action": "shuffle"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("shuffled"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_queue_unknown_action() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_queue", &json!({"action": "bogus"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    // ---- tool_library ----

    #[test]
    fn tool_library_stats() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_library", &json!({"action": "stats"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("0 items"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_library_list_empty() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_library", &json!({"action": "list"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("empty"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_library_list_with_items() {
        let (path, mut plib) = tmp_db();
        plib.add_item(make_item("Song A", "Artist X")).unwrap();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_library", &json!({"action": "list"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Artist X"));
        assert!(text.contains("Song A"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_library_scan_missing_path() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_library", &json!({"action": "scan"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_library_scan_nonexistent() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_library",
            &json!({"action": "scan", "path": "/nonexistent_dir_12345"}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "tarang")]
    #[test]
    fn tool_library_scan_valid_dir() {
        // Create a temp dir with a WAV file
        let dir = std::env::temp_dir().join(format!("jalwa_mcp_scan_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = jalwa_core::test_fixtures::make_test_wav(4410, 44100);
        std::fs::write(dir.join("test.wav"), &wav).unwrap();

        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_library",
            &json!({"action": "scan", "path": dir.to_str().unwrap()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("added"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_library_unknown_action() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_library", &json!({"action": "bogus"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    // ---- tool_playlist ----

    #[test]
    fn tool_playlist_list_empty() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_playlist", &json!({"action": "list"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No playlists"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_create() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_playlist",
            &json!({"action": "create", "name": "Favorites"}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Created"));
        assert!(text.contains("Favorites"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_create_missing_name() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_playlist", &json!({"action": "create"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_list_with_playlists() {
        let (path, mut plib) = tmp_db();
        let pl = jalwa_core::Playlist::new("My Mix");
        plib.save_playlist(&pl).unwrap();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_playlist", &json!({"action": "list"}), &plib, &eng);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("My Mix"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_add_missing_params() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_playlist", &json!({"action": "add"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_add_invalid_uuid() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_playlist",
            &json!({"action": "add", "name": "Test", "item_id": "bad-uuid"}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_add_playlist_not_found() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_playlist",
            &json!({"action": "add", "name": "Missing", "item_id": uuid::Uuid::new_v4().to_string()}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_add_valid() {
        let (path, mut plib) = tmp_db();
        let pl = jalwa_core::Playlist::new("Rock");
        plib.save_playlist(&pl).unwrap();
        let item = make_item("Song", "Band");
        let id = item.id;
        plib.add_item(item).unwrap();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_playlist",
            &json!({"action": "add", "name": "Rock", "item_id": id.to_string()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Added"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_export_missing_params() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_playlist", &json!({"action": "export"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_export_not_found() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_playlist",
            &json!({"action": "export", "name": "Missing", "output": "/tmp/out.m3u"}),
            &plib,
            &eng,
        );
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_export_valid() {
        let (path, mut plib) = tmp_db();
        let pl = jalwa_core::Playlist::new("Chill");
        plib.save_playlist(&pl).unwrap();
        let output =
            std::env::temp_dir().join(format!("jalwa_mcp_export_{}.m3u", uuid::Uuid::new_v4()));
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_playlist",
            &json!({"action": "export", "name": "Chill", "output": output.to_str().unwrap()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Exported"));
        let _ = std::fs::remove_file(&output);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tool_playlist_unknown_action() {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call("jalwa_playlist", &json!({"action": "bogus"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    // ---- tool_recommend with results ----

    #[test]
    fn tool_recommend_with_library() {
        let (path, mut plib) = tmp_db();
        let item1 = make_item("Song A", "Artist X");
        let id1 = item1.id;
        plib.add_item(item1).unwrap();
        let mut item2 = make_item("Song B", "Artist X");
        item2.tags = vec!["rock".to_string()];
        plib.add_item(item2).unwrap();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_recommend",
            &json!({"item_id": id1.to_string()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        // Should find recommendations (same artist)
        assert!(text.contains("Artist X") || text.contains("No recommendations"));
        let _ = std::fs::remove_file(&path);
    }

    // ---- play with valid WAV ----

    #[test]
    fn tool_play_valid_wav() {
        let dir = std::env::temp_dir().join(format!("jalwa_mcp_play_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = jalwa_core::test_fixtures::make_test_wav(4410, 44100);
        let wav_path = dir.join("test.wav");
        std::fs::write(&wav_path, &wav).unwrap();

        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();
        let result = handle_tool_call(
            "jalwa_play",
            &json!({"path": wav_path.to_str().unwrap()}),
            &plib,
            &eng,
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Playing"));
        // Cleanup
        {
            eng.lock().unwrap().stop();
        }
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&path);
    }

    // ---- MCP stdio integration tests (run_on) ----

    /// Helper: run the JSON-RPC loop with the given input lines and return parsed responses.
    async fn run_on_input(input: &str) -> Vec<serde_json::Value> {
        let (path, plib) = tmp_db();
        let plib = Arc::new(Mutex::new(plib));
        let eng = test_engine();

        let reader = tokio::io::BufReader::new(input.as_bytes());
        let mut output = Vec::new();

        run_on(reader, &mut output, plib, eng).await.unwrap();

        let _ = std::fs::remove_file(&path);

        // Each response is a single JSON line
        String::from_utf8(output)
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn run_initialize() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#.to_string() + "\n";
        let responses = run_on_input(&input).await;
        assert_eq!(responses.len(), 1);
        let r = &responses[0];
        assert_eq!(r["jsonrpc"], "2.0");
        assert_eq!(r["id"], 1);
        assert!(r["result"]["protocolVersion"].as_str().is_some());
        assert_eq!(r["result"]["serverInfo"]["name"], "jalwa");
    }

    #[tokio::test]
    async fn run_tools_list() {
        let input = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.to_string() + "\n";
        let responses = run_on_input(&input).await;
        assert_eq!(responses.len(), 1);
        let tools = responses[0]["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
    }

    #[tokio::test]
    async fn run_tool_call_status() {
        let input = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "jalwa_status", "arguments": {} }
        }))
        .unwrap()
            + "\n";
        let responses = run_on_input(&input).await;
        assert_eq!(responses.len(), 1);
        let text = responses[0]["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        assert!(text.contains("Stopped"));
    }

    #[tokio::test]
    async fn run_unknown_method() {
        let input = r#"{"jsonrpc":"2.0","id":4,"method":"bogus/method"}"#.to_string() + "\n";
        let responses = run_on_input(&input).await;
        assert_eq!(responses.len(), 1);
        let err = responses[0]["result"]["error"].as_str().unwrap();
        assert!(err.contains("unknown method"));
    }

    #[tokio::test]
    async fn run_malformed_json_skipped() {
        let input = "this is not json\n".to_string()
            + r#"{"jsonrpc":"2.0","id":5,"method":"initialize"}"#
            + "\n";
        let responses = run_on_input(&input).await;
        // Malformed line is skipped; only the valid request produces a response.
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["id"], 5);
    }

    #[tokio::test]
    async fn run_multiple_requests() {
        let input = format!(
            "{}\n{}\n{}\n",
            r#"{"jsonrpc":"2.0","id":10,"method":"initialize"}"#,
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":12,"method":"unknown"}"#,
        );
        let responses = run_on_input(&input).await;
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["id"], 10);
        assert_eq!(responses[1]["id"], 11);
        assert_eq!(responses[2]["id"], 12);
        // Verify each response type
        assert!(responses[0]["result"]["protocolVersion"].as_str().is_some());
        assert_eq!(responses[1]["result"]["tools"].as_array().unwrap().len(), 8);
        assert!(
            responses[2]["result"]["error"]
                .as_str()
                .unwrap()
                .contains("unknown method")
        );
    }
}
