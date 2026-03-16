# Jalwa MCP Tools

## Tools

### jalwa_play
Play a media file (audio or video).
**Input**: `{ "path": "/path/to/file.mp3" }`

### jalwa_pause
Pause current playback.
**Input**: `{}`

### jalwa_status
Get current playback status (state, position, volume, current track).
**Input**: `{}`

### jalwa_search
Search the media library by title, artist, album, or tag.
**Input**: `{ "query": "queen bohemian" }`

### jalwa_recommend
Get AI-powered media recommendations based on a seed item.
**Input**: `{ "item_id": "uuid-here", "max": 5 }`
