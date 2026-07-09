# jalwa — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** —
> durable rules that change rarely. Volatile state (current version,
> module line counts, port progress, test counts) lives in
> [`docs/development/state.md`](docs/development/state.md). Do not inline state here.

## Project Identity

**jalwa** — Cyrius port of a Rust project (full 5-crate workspace preserved at `rust-old/`).

- **Type**: Port (Rust → Cyrius)
- **License**: GPL-3.0-only
- **Language**: Cyrius (toolchain pinned in `cyrius.cyml [package].cyrius`)
- **Version**: `VERSION` at the project root is the source of truth — do not inline the number here
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md) · [First-Party Documentation](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

## Goal

jalwa is the **AI-native media player for AGNOS** — media library (index, search,
smart playlists), playback (decode → DSP/EQ/loudness → audio out), a terminal UI,
a Wayland desktop GUI, AI recommendations/transcription via hoosh, and an MCP server.
It sits on the **tarang** media framework and drives **dhvani** for audio DSP.

## The Port

**The plan of record is [`docs/development/port-audit.md`](docs/development/port-audit.md)** —
the playbook (Rust→Cyrius conventions), the workspace-collapse strategy, the subsystem→Cyrius
wiring table, the blockers, the wave plan, and the per-module ledger. Read it before porting.

Decisions in force (see port-audit + [ADRs](docs/adr/)):

- **tarang is stubbed** — it's still Rust; port call sites against a stub, no real playback in v1.
- **GUI is in scope** as a dhancha/mabda **Wayland** app (reference: the `puka` repo).
- **Video is backlogged** — needs tarang + aethersafta (both still Rust).
- **MPRIS export is deferred** — `samvada` is D-Bus client-only.

## Scaffolding

Scaffolded with `cyrius port`. The **full 5-crate Rust workspace** is frozen at `rust-old/`
as the reference oracle — do not modify it; cross-check the port against it.

## Quick Start

```sh
cyrius lib sync                          # vendor declared [deps].stdlib into lib/ from the pin
cyrius deps                              # resolve [deps.X] repo deps (dhvani/bote/…) into lib/, write cyrius.lock
cyrius build src/main.cyr build/jalwa    # compile
cyrius test tests/error.tcyr             # run one suite (bare `cyrius test` runs all tests/*.tcyr — what CI does)
```

> **Vendoring**: `cyrius lib sync` vendors stdlib modules (adding one to `cyrius.cyml [deps].stdlib`
> then running `cyrius deps` fails — `deps` is only for repo `[deps.X]`). Basic `f64_*` ops are
> compiler builtins; `math`/`ganita` are for higher-level math.

## Key Principles

- **Cross-check against `rust-old/`** — the port's bar is "matches what Rust did". Diverge only with an ADR.
- **Correctness over cleverness** — a silent divergence from Rust means the bugs win.
- **Alloc-free hot paths** — the bump allocator never frees; a per-sample/per-block `alloc()`
  leaks across a render. Reuse struct-owned scratch, write to caller out-buffers.
- **Integer literals are NOT f64** — write f64 args as decimals (`1.0`, not `1`); keep counts integer.
- Test after every change, not after the feature is "done". ONE change at a time.
- Build with `cyrius build` (the manifest auto-resolves deps), not raw `cat | cc5`.
- Source files only need project includes — stdlib auto-resolves from `cyrius.cyml`.
- `.cyr` files never `include` each other — the entry file / `[lib].modules` imposes order.
- `var buf[N]` = N **bytes**, not N entries.
- **Prefix every symbol** `jlw_…`/`Jlw…`/`JLW_…` (flat namespace shared with sibling dist bundles).

## Toolchain concurrency

Every `cyrius build/test/deps` re-resolves deps and races on `cyrius.lock` — concurrent runs
corrupt it. Serialize parallel toolchain calls behind a file lock:
`flock <scratch>/jalwa-build.lock cyrius test …`.

## Rules (Hard Constraints)

- **Do not commit or push** — the user handles all git operations
- **Never use `gh` CLI** — use `curl` to the GitHub API if needed
- **`/home/macro/Repos/cyrius` is READ-ONLY** — reference only; file an issue (`#date-item`)
  with permission if a language gap is found. Never write there.
- Do not modify `rust-old/` — it's the parity oracle
- Do not skip tests before claiming changes work
- Do not modify `lib/` files (vendored stdlib / dep symlinks)
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the source of truth

## Reference repos

- **`dhvani`** — the gold-standard completed port (audio engine). Methodology template.
- **`vidya`** — Cyrius language + cross-language reference corpus (`content/`).
- **`puka`** — Cyrius Wayland/mabda app; the GUI reference for `jalwa-gui`.
- **`bote`** — MCP core; backs `src/mcp.cyr`.

## Documentation

- [`docs/adr/`](docs/adr/) — Architecture Decision Records (*why X over Y?*)
- [`docs/development/port-audit.md`](docs/development/port-audit.md) — **the port plan**
- [`docs/development/state.md`](docs/development/state.md) — live state
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — wave sequencing + backlog
