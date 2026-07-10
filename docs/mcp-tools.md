# Jalwa MCP Tools

`jalwa mcp` is a stdio JSON-RPC 2.0 server exposing the 8 tools below. Tool
args arrive as compact JSON; string fields (and the optional numeric `max`) are
read from the object. UUIDs are bare 32-hex-char strings.

## Tools

### jalwa_play
Open and play a media file (audio only — video is P1-blocked in v1).
**Input**: `{ "path": "/path/to/file.mp3" }`

### jalwa_pause
Pause current playback (returns the updated status).
**Input**: `{}`

### jalwa_status
Get current playback status (state and volume).
**Input**: `{}`

### jalwa_search
Search the media library by title, artist, album, or tag.
**Input**: `{ "query": "queen bohemian" }`

### jalwa_recommend
Get AI-powered media recommendations based on a seed item. `max` is optional
(defaults to 5).
**Input**: `{ "item_id": "32-hex-uuid", "max": 5 }`

### jalwa_queue
Inspect or modify the playback queue. `action` is one of `list`, `enqueue`
(needs `item_id`), `clear`, or `shuffle`.
**Input**: `{ "action": "enqueue", "item_id": "32-hex-uuid" }`

### jalwa_library
Query or grow the library. `action` is one of `stats`, `scan` (needs `path`),
or `list`.
**Input**: `{ "action": "scan", "path": "/music" }`

### jalwa_playlist
Manage playlists. `action` is one of `list`, `create` (needs `name`), or `add`
(needs `name` and `item_id`).
**Input**: `{ "action": "add", "name": "Favs", "item_id": "32-hex-uuid" }`
