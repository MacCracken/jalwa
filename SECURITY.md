# Security Policy

## Scope

Jalwa is an AI-native media player built on the Tarang media framework. It handles local media files, audio output via PipeWire, and optional D-Bus/MPRIS integration.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Media file parsing | Malicious containers triggering decoder bugs | Tarang demuxers validate structure; codec FFI boundaries are safe-wrapped |
| SQLite library DB | SQL injection via crafted metadata | All queries use parameterized placeholders |
| MCP server | Malformed JSON-RPC input | Input validated before dispatch; bounded input size |
| D-Bus/MPRIS | Unauthorized playback control | System D-Bus access controls apply |
| File scanning | Symlink loops / path traversal | WalkDir with max depth; paths resolved before opening |
| Album art cache | Memory exhaustion via many unique art entries | LRU eviction bounds cache size |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2026.3.x | Yes |
| < 2026.3 | No |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours
- We follow a 90-day coordinated disclosure timeline
- Please do not open public issues for security vulnerabilities

## Design Principles

- Safe Rust throughout — no `unsafe` code in jalwa crates
- Codec FFI (dav1d, openh264) wrapped behind safe tarang boundaries
- Parameterized queries for all SQL
- No network I/O in core playback path
- Feature-gated video codecs to minimize attack surface
