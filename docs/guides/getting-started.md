# Getting started with jalwa

jalwa is a **Cyrius** port of a Rust media player. There are no crates or cargo —
modules are `.cyr` files under `src/`, and the original Rust workspace is frozen at
`rust-old/` as a read-only parity oracle.

## Build

```sh
cyrius lib sync                          # vendor declared [deps].stdlib into lib/ from the pin
cyrius deps                              # resolve [deps.X] repo deps (dhvani/shravan/vani/…) into lib/, write cyrius.lock
cyrius build src/main.cyr build/jalwa    # compile
cyrius test                              # run tests/*.tcyr (bare = what CI does)
```

> Concurrent `cyrius build/test/deps` race on `cyrius.lock`; serialize parallel
> runs behind `flock <scratch>/jalwa-build.lock cyrius test …`.

## Run

`build/jalwa <command>`. Working subcommands:

- `scan` — index a media directory (real audio probe + tags)
- `play` — decode → EQ/normalize → audio out (WAV/FLAC/MP3)
- `info` — show format/duration/codec/tags for a file
- `search`, `stats`, `library`, `export`, `import`, `devices`
- `tui` — interactive terminal UI
- `gui` — Wayland present shell (validate on real AGNOS; smoke-only headless)
- `mcp` — stdio JSON-RPC 2.0 MCP server (8 tools)

Video is deferred (P1) — blocked on tarang/aethersafta (both still Rust).

## Layout

- `src/main.cyr` — entry point + CLI dispatch. Top-level `var r = main(); syscall(SYS_EXIT, r);`.
- `src/*.cyr` — ported modules (core, playback, ai, ui, gui, mcp). `.cyr` files never
  `include` each other; the entry file / `[lib].modules` imposes order.
- `tests/` — test suite (`.tcyr` files, auto-discovered by `cyrius test`).
- `rust-old/` — original Rust source preserved for parity checks. Do not modify; it's the reference oracle.

## Adding a feature

1. Edit `src/main.cyr` (or add a new module and wire it into the entry file / `[lib].modules`).
2. Cross-check parity against `rust-old/`.
3. Add a per-module `.tcyr` suite under `tests/`.
4. Run `cyrius test`.
5. Bump `VERSION` and add a CHANGELOG entry before tagging.

See [`../adr/template.md`](../adr/template.md) when a non-trivial design choice deserves an ADR.
