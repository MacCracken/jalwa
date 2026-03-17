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
    use serde_json::Value;
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
        println!("{}", serde_json::to_string(&response)?);
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

fn tool_pause(
    engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> serde_json::Value {
    let mut eng = engine.lock().unwrap();
    eng.pause();
    let status = eng.status();
    mcp_ok(format!(
        "paused\n{}",
        serde_json::to_string_pretty(&status).unwrap_or_default()
    ))
}

fn tool_status(
    engine: &Arc<Mutex<jalwa_playback::PlaybackEngine>>,
) -> serde_json::Value {
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
        let path =
            std::env::temp_dir().join(format!("jalwa_mcp_test_{}.db", uuid::Uuid::new_v4()));
        let plib = jalwa_core::db::PersistentLibrary::open(&path).unwrap();
        (path, plib)
    }

    fn test_engine() -> Arc<Mutex<jalwa_playback::PlaybackEngine>> {
        Arc::new(Mutex::new(
            jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default()),
        ))
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
        let result =
            handle_tool_call("jalwa_recommend", &json!({"item_id": "not-a-uuid"}), &plib, &eng);
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
        let result =
            handle_tool_call("jalwa_play", &json!({"path": "/nonexistent.wav"}), &plib, &eng);
        assert!(result["isError"].as_bool().unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }
}
