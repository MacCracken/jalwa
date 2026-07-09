# jalwa — Port Audit (Rust → Cyrius)

> The **playbook + ledger** for porting jalwa from Rust to Cyrius. Durable
> conventions live here; live progress lives in [`state.md`](state.md); the
> release sequencing lives in [`roadmap.md`](roadmap.md).
>
> Scaffolded with `cyrius port` on 2026-07-09. The full 5-crate Rust workspace
> (**15,355 LOC**, 40 `.rs` files) is frozen at [`../../rust-old/`](../../rust-old/)
> as the **parity oracle** — never edit it; cross-check every ported module against it.

## What jalwa is

AI-native media player for AGNOS, built (in Rust) as a 5-crate Cargo workspace on
the **tarang** media framework. The Cyrius port collapses the workspace into one
flat `src/` of `.cyr` modules (Cyrius has no crates/modules/namespaces — one flat
symbol table per compilation unit).

| Rust crate | LOC | Owns | Cyrius target dir |
|---|---:|---|---|
| `jalwa-core` | ~2,600 | library, DB, scanner, watcher, playlists, queue, state, hardware | `src/core/` |
| `jalwa-playback` | ~2,700 | tarang decode pipeline, DSP/EQ, PipeWire out, MPRIS, video decode | `src/playback/` |
| `jalwa-ai` | ~1,460 | recommendations, smart playlists, daimon RAG, fingerprint | `src/ai/` |
| `jalwa-ui` | ~1,980 | terminal UI (ratatui) + string renderers | `src/ui/` |
| `jalwa-gui` | ~3,340 | desktop GUI (egui/eframe) — 8 views, art cache, theme | `src/gui/` |
| root bin | ~2,080 | `main.rs` CLI + `mcp.rs` MCP server | `src/main.cyr`, `src/mcp.cyr` |

## Decisions in force (2026-07-09)

- **tarang → STUB.** tarang (media framework: decode/probe/`AudioBuffer`/`MediaInfo`/
  fingerprint) is **still Rust** and used by all 5 crates. Port every `use tarang::`
  call site against a **stubbed tarang surface** so structure is preserved and code
  compiles, but real decode/playback is non-functional until a Cyrius tarang lands.
  Do **not** build a facade or refactor to siblings yet.
- **GUI → in scope for v1** as a **Wayland/Linux** app: port `jalwa-gui` from egui
  immediate-mode to **dhancha** retained-mode, using **[puka](../../../puka)** (mabda-based
  Cyrius Wayland app) as the reference for window + GPU + event loop.
- **Video → backlogged.** `video_decode_thread.rs` and `view_video.rs` are **not** in
  the v1 port table — they need tarang **and** `aethersafta` (also still Rust). Roadmapped.
- **MPRIS → deferred.** No server-side D-Bus in Cyrius (`samvada` is client-only). Port
  the command/state shape; defer the object export (see Blockers).

## The port playbook (Rust → Cyrius mapping rules)

Mirrors the dhvani conventions (see `../../../dhvani/docs/development/port-audit.md`).

- **Symbol prefix (flat namespace).** Every top-level symbol is collision-proofed with
  a crate prefix. Free fns `jlw_<crate>_<verb>`, structs `Jlw…` CamelCase, consts
  `JLW_…`. Crate → prefix map:
  - `jalwa-core` → `jlw_core_` / `JlwCore…` / `JLW_CORE_`
  - `jalwa-playback` → `jlw_pb_` / `JlwPb…`
  - `jalwa-ai` → `jlw_ai_` / `JlwAi…`
  - `jalwa-ui` → `jlw_ui_` / `JlwUi…`
  - `jalwa-gui` → `jlw_gui_` / `JlwGui…`
  - root bin → `jlw_` / `jlw_mcp_` / `jlw_main_`
- **enums → integer `var`/`enum` codes + if/elif dispatch.** No match-on-variant.
  (`MediaType`, `PlaybackState`, `RepeatMode`, `AudioCodec`, `ContainerFormat`,
  `VideoCodec`, `View`, `InputMode`, `RecommendationReason`, `EngineCommand`, …)
  `Display` impls → a `_name(code)` fn returning a string literal.
- **error enum → `JLW_ERR_*` codes** in `src/error.cyr` (included FIRST by every entry):
  `JLW_ERR_NONE=0`, negative codes per `JalwaError` variant, `jlw_is_err(code)=code<0`,
  `jlw_err_name(code)`. `thiserror` `#[from]` context is dropped.
- **`Result<T>`/`Option<T>` → sentinel returns** (no unwinding, no panic): error code
  for validators; `0`/null-handle for fallible constructors; `-1` for absent index;
  `NaN` for `Option<f64>` where the valid range excludes NaN. Real multi-field payloads
  ride the `tagged`/`result` stdlib.
- **closures/`FnMut` → fn-ptr + opaque state** via `fnptr` (`&fn_name`, `fncall0..8`);
  trait objects (e.g. an `AudioNode`) → same fn-ptr + state pattern. Iterator adapters
  (`.iter().map()/.fold()/.collect()`) → explicit index `while` loops.
- **`Vec`/`SmallVec` → stdlib `vec`**; nested `Vec<Vec<T>>` → one flat arena indexed
  `out*N+in`. `f64` elements store directly in the 8-byte slots. `var buf[N]` = N **bytes**.
- **structs → `#derive(accessors) struct` + `alloc(sizeof(T))`**; `#[derive(Clone)]` → a
  hand-written deep-copy fn.
- **f32 → f64 everywhere** in the DSP/audio path (dhvani/ganita math is f64-only).
- **Float-literal trap:** an integer literal like `120` is **not** `120.0` as f64 — write
  f64 args as decimals; keep integer-count args (sample_rate, frames, channels) integer.
  Non-integer constants that need exactness use a module-top `var` holding the IEEE-754 hex.
- **serde → hand-written `to_json`/`from_json` over `bayan`.** All serde round-trip and
  `Display`-format tests are dropped; every other `#[test]` ports one-for-one into
  `tests/<mod>.tcyr`, cross-checked against `rust-old/`.
- **Alloc-free hot paths.** The bump allocator has no per-object free — a per-sample/
  per-block `alloc()` leaks unboundedly across a render. Every process loop allocates
  zero bytes/sample; reuse struct-owned scratch, write results to caller out-buffers.
- **No Cargo features.** Feature-gated code is either always-on or dropped; genuine
  platform splits use `#ifdef CYRIUS_TARGET_LINUX` (e.g. inotify).

## Dependency wiring (subsystem → Cyrius backing)

Most subsystems map to **stdlib distlibs** (auto-resolve via the `cyrius.cyml [deps].stdlib`
list — no git wiring). Only the non-stdlib repos need explicit `[deps.X] modules=["dist/X.cyr"]`.

| Rust dep | Subsystem | Cyrius backing | Kind | Notes |
|---|---|---|---|---|
| `rusqlite` | media-library DB | **patra** | stdlib | SQL subset (limited JOINs) — fine for the schema |
| `lofty` | audio tag read | **shravan** (`dist/shravan.cyr`) | repo | ID3v2 + Vorbis text; **gap**: embedded art, MP4/APEv2 |
| `serde_json` | JSON | **bayan** | stdlib | also toml/cyml/csv/base64 |
| `reqwest` | HTTP client | **net** + **http** | stdlib | `http` is GET-focused — add POST+body for daimon |
| `image` | JPEG/PNG decode | **chitra** (`dist/chitra.cyr`) | repo | RGBA8; resize is manual/`sadish` |
| `egui`/`eframe` | desktop GUI | **dhancha** + **mabda** (GPU) + **rekha**/**sadish** | repo/stdlib | retained-mode rewrite; Wayland; puka pattern |
| `ratatui` | TUI widgets | *hand-rolled ANSI CSI* | — | no lib; reference `thoth`/`chakshu` |
| `crossterm` | tty control | **darshana** (`dist/darshana.cyr`) | repo | raw/alt/move/winsize direct |
| PipeWire | audio output | **vani** | stdlib | ALSA PCM + ring buffer + mixer |
| `tracing` | logging | **sakshi** / **log** | stdlib | |
| `notify` | fs watch | raw **inotify** syscalls | stdlib | degrade to periodic `dir_walk` if absent |
| `walkdir` | dir traversal | **fs** (`dir_walk`/`find_files`) | stdlib | direct |
| `chrono` | datetime | **chrono** | stdlib | epoch i64 + civil-time helpers |
| `uuid` | v4 ids | **random** (getrandom) | stdlib | 16 bytes + version/variant bits |
| `clap` | CLI parse | **args** | stdlib | hand-rolled subcommand dispatch |
| `tokio` | async | **thread** (blocking) or **async** (epoll, `CYRIUS_ASYNC=1`) | stdlib | not tokio-parity |
| `rayon` | data parallel | **thread** fan-out + **atomic** | stdlib | |
| — | MCP core | **bote** (`dist/bote.cyr`, 3.x) | repo | JSON-RPC/tools |
| — | math | **abaco** (`dist/abaco.cyr`) | repo | dhvani already externalizes it |
| — | audio engine | **dhvani** (`dist/dhvani.cyr`, 2.x) | repo | EQ/loudness/analysis — ✅ ported |
| — | HW accel | **ai-hwaccel** | repo | ✅ ported |
| — | device abstraction | **yukti** | stdlib | ✅ folded distlib (USB/optical/udev) — resolves via stdlib list |
| — | D-Bus (client) | **samvada** (`dist/samvada.cyr`) | repo | client-only — **no** MPRIS server |

## Blockers

1. **tarang (media framework) — still Rust, no dist.** The linchpin: probe/decode/
   `AudioBuffer`/`MediaInfo`/fingerprint, consumed by all 5 crates. **Strategy: stub.**
   Every tarang call site is ported against a stub surface; real playback unblocks when a
   Cyrius tarang (or a facade over shravan+dhvani+vani) exists.
2. **aethersafta (video surface/compositor) — still Rust.** The Cyrius `aethersafha` is a
   compositor *app*, not a linkable lib. Blocks the entire video path → **backlogged**.
3. **MPRIS / D-Bus export.** `samvada` is client-only (logind-scoped, C-shim), no object
   export / property marshalling. Port the `MprisCommand` shape; **defer** the export.
   Later: extend `samvada`, or use native IPC (`majra` pub/sub or a `bote`/`setu` channel).
4. **tarang-ai fingerprint kernel.** `compute_fingerprint`/`fingerprint_match` have no
   Cyrius target — port the parallel structure, stub the kernel (or bridge dhvani
   chroma/onset/beat).
5. **`ratatui` widgets** — no Cyrius TUI lib; ~1,350 LOC of widgets+tui hand-rolled as
   ANSI CSI (tractable, not a hard blocker; reference `thoth`/`chakshu`).

## Wave plan

Dependency-ordered. Waves gated only by decided blockers; tarang-independent modules first.
Waves C and D are parallelizable after A. Each module is verified against `rust-old/` before merge.

| Wave | Title | Modules | Gate |
|---|---|---|---|
| **0** | Scaffold + oracle freeze + deps wiring | `cyrius port` ✅; stdlib deps (incl. `yukti`, `patra`, `vani`, `mabda`, `bayan`, …); dist wiring (dhvani/ai-hwaccel/bote/shravan/chitra/dhancha/darshana/abaco); `src/error.cyr` (`JLW_ERR_*`) | — |
| **A** | Core domain types (L0) | `jalwa-core/lib.rs` → `src/core/types.cyr` (+ `error.cyr`); enums→codes; uuid→random; chrono | Wave 0 |
| **B** | Core services (L1) | `db.rs`→patra, `playlist_io.rs`→fs (easiest), `scanner.rs`→fs+shravan (tarang stub), `watcher.rs`→inotify, `hardware.rs`→yukti | Wave A |
| **C** | Playback + DSP (L2a) | `dsp.rs`→dhvani (clean map, do FIRST), `decode_thread.rs` (tarang stub), `lib.rs`→`engine.cyr`; `mpris.rs` shape-only; `video_decode_thread.rs` deferred | Wave A + dhvani |
| **D** | AI features (L2b) | `ai/lib.rs`→`reco.cyr` (pure, direct), `daimon.rs`→net/http+bayan, `fingerprint.rs` (thread fan-out, stub kernel) | Wave A (∥ C) |
| **E** | Terminal UI (L3a) | `ui/lib.rs`→`renderers.cyr` (pure, early win), `app.rs`, `widgets.rs`+`tui.rs`→darshana + hand-rolled CSI | Wave A + C |
| **F** | Desktop GUI (L3b) | `theme.rs`, `art_cache.rs`→chitra, 8 `views/*`→dhancha widget trees, `app.rs`, `lib.rs`→`run.cyr` (mabda/Wayland, puka pattern); `view_video` deferred | Wave A + C |
| **G** | Root bin + MCP (L4) | `mcp.rs`→bote, `main.rs`→args dispatch; wire full `[lib].modules` order; integration suites | all prior |

## Module ledger

| Rust module (LOC) | → Cyrius | Layer | Parity notes |
|---|---|---|---|
| `jalwa-core/lib.rs` (1026) | `core/types.cyr` + `error.cyr` | L0 | structs→`#derive(accessors)`; enums→codes; **drop** serde derives/tests; `JalwaError`→`JLW_ERR_*`; uuid→random, chrono→epoch; tarang `MediaInfo` re-export via fallback enums |
| `jalwa-core/db.rs` (852) | `core/db.cyr` | L1 | rusqlite→**patra** open/prepare/bind/step; serde rows→manual bayan mapping; SQL subset OK; long pole of B |
| `jalwa-core/scanner.rs` (397) | `core/scanner.cyr` | L1 | walkdir→**fs** `dir_walk`; lofty→**shravan** (MP3/FLAC/Ogg text); **gap** MP4/APE/art; tarang probe **stubbed** |
| `jalwa-core/watcher.rs` (282) | `core/watcher.cyr` | L1 | notify→raw **inotify** syscalls + debounce; events→codes over `thread` chan; degrade to polling if inotify absent |
| `jalwa-core/playlist_io.rs` (174) | `core/playlist_io.cyr` | L1 | **easiest** — std fs M3U ↔ **fs**/**io**; no ext dep |
| `jalwa-core/hardware.rs` (896) | `core/hardware.cyr` | L1 | →**yukti** device/storage/optical/udev; tracing→sakshi |
| `jalwa-playback/dsp.rs` (503) | `playback/dsp.cyr` | L2 | **cleanest map** — 10-band EQ→`dhvani_graphic_eq_*`, loudness→dhvani `normalize_to_lufs`; widen f32↔f64 at tarang bridge; **alloc-free** render loop; do FIRST |
| `jalwa-playback/decode_thread.rs` (671) | `playback/decode_thread.cyr` | L2 | resample/mix/volume/EQ pipeline over dhvani+vani; commands→codes over `thread` chan; tarang `FileDecoder` **stubbed** |
| `jalwa-playback/video_decode_thread.rs` (376) | `playback/video_decode_thread.cyr` | L2 | **DEFER** — tarang Demuxer + aethersafta; port thread scaffold only |
| `jalwa-playback/mpris.rs` (314) | `playback/mpris.cyr` | L2 | **BLOCKER** — `MprisCommand`→codes; **defer** D-Bus export (samvada client-only) |
| `jalwa-playback/lib.rs` (808) | `playback/engine.cyr` | L2 | `PlaybackEngine` facade→`jlw_pb_engine_*`; `format_duration` direct; ported LAST in C |
| `jalwa-ai/daimon.rs` (818) | `ai/daimon.cyr` | L2 | reqwest POST→**net/http**+**bayan** (add POST helper); tokio→**thread**/async; rustls dropped (localhost) |
| `jalwa-ai/fingerprint.rs` (196) | `ai/fingerprint.cyr` | L2 | rayon→**thread** fan-out+**atomic**; kernel **stubbed** (tarang-ai) |
| `jalwa-ai/lib.rs` (442) | `ai/reco.cyr` | L2 | **direct** — smart-playlist rules over Library; reasons→codes; the AI safe core |
| `jalwa-ui/lib.rs` (260) | `ui/renderers.cyr` | L3 | **direct, early win** — stateless string renderers over `str` builders |
| `jalwa-ui/app.rs` (375) | `ui/app.cyr` | L3 | `App` state→`#derive(accessors)`; View/InputMode→codes |
| `jalwa-ui/widgets.rs` (586) | `ui/widgets.cyr` | L3 | **hand-roll** ratatui→ANSI CSI cell-painting |
| `jalwa-ui/tui.rs` (759) | `ui/tui.cyr` | L3 | crossterm→**darshana**; Frame/Layout→bespoke double-buffer/cell-diff; MPRIS wiring deferred |
| `jalwa-gui/theme.rs` (74) | `gui/theme.cyr` | L3 | egui `Visuals`→dhancha style consts |
| `jalwa-gui/art_cache.rs` (274) | `gui/art_cache.cyr` | L3 | image→**chitra** RGBA8; LRU+texture→dhancha; art via shravan (needs art support) |
| `jalwa-gui/views/mod.rs` (8) | *dropped* | L3 | no module system |
| `jalwa-gui/views/library.rs` (476) | `gui/view_library.cyr` | L3 | largest view — list/grid/search→dhancha widget tree |
| `jalwa-gui/views/now_playing.rs` (230) | `gui/view_now_playing.cyr` | L3 | art + track detail→dhancha image/text |
| `jalwa-gui/views/queue.rs` (208) | `gui/view_queue.cyr` | L3 | queue list + reorder→dhancha list |
| `jalwa-gui/views/transport.rs` (212) | `gui/view_transport.cyr` | L3 | play/seek/volume→dhancha buttons/slider |
| `jalwa-gui/views/equalizer.rs` (105) | `gui/view_equalizer.cyr` | L3 | 10-band sliders→dhancha sliders → engine EqSettings |
| `jalwa-gui/views/sidebar.rs` (92) | `gui/view_sidebar.cyr` | L3 | nav buttons; View→codes |
| `jalwa-gui/views/video.rs` (110) | `gui/view_video.cyr` | L3 | **DEFER** — needs blocked video path |
| `jalwa-gui/views/devices.rs` (64) | `gui/view_devices.cyr` | L3 | yukti device list→dhancha list |
| `jalwa-gui/app.rs` (580) | `gui/app.cyr` | L3 | **rewrite** — eframe `update()` immediate-mode → dhancha retained tree + per-frame diff |
| `jalwa-gui/lib.rs` (33) | `gui/run.cyr` | L3 | eframe launch → dhancha/mabda Wayland event loop (puka pattern) |
| `src/mcp.rs` (1281) | `src/mcp.cyr` | L4 | →**bote** ToolDef/ToolSchema; 8 JSON-RPC tools over stdio; JSON via bayan; only `jlw_ai_recommend` caller |
| `src/main.rs` (795) | `src/main.cyr` | L4 | clap→**args** dispatch + `--help`/`--version`; libc signals→`rt_sigaction`; tarang probe stubbed; the `[build]` entry |

## Top risks

- **tarang stubbing means no real playback in v1** — the whole decode/probe/fingerprint
  surface is stubbed. v1 = green code + correct structure, not a working player.
- **GUI is a full rewrite** (~3,340 LOC egui→dhancha), the largest lift and top schedule risk.
- **Silent float traps** in the DSP path: integer-literal-≠-f64, f32→f64 tolerances, and a
  leaking bump-alloc in the render loop (passes tests, unusable in a real render).
- **serde has no derive** — every serde struct (MediaItem/Playlist/Library + AI bodies)
  needs hand-written `to_json`/`from_json`; a missed field silently corrupts DB rows/requests.
- **LEXID cap** — linking too many dist bundles at once can overflow the compiler's
  65,536-entry LEXID cap. Assemble per-feature subsets; vendor externalized siblings in order.
- **Version drift** — jalwa's Rust `Cargo.toml` pins old sibling versions (dhvani 0.20.4,
  yukti 0.22.3, ai-hwaccel 0.21.3); on-disk Cyrius dists are far ahead. Port against the
  **current Cyrius dist API**, not the old Rust API the oracle was written against.

## Open decisions (unresolved)

- **DB:** confirm `patra`'s SQL subset covers the media-library schema (no unsupported JOINs).
- **AGNOS syscall surface:** does the frozen kernel expose getrandom (uuid), inotify
  (watcher), epoll + monotonic clock (async)? Each gates a `#ifdef` fallback.
- **shravan tag scope:** extend to lofty parity (embedded cover-art, MP4/APEv2, arbitrary
  tags) — needed by `art_cache`/`scanner` — or scope rich tags/art out of v1?
- **`http_post` ownership:** add a reusable POST+JSON helper to stdlib `http`/`net`
  (bote already POSTs JSON-RPC) or keep a jalwa-local helper?
- **MPRIS:** required on AGNOS, or use an AGNOS-native media-control convention
  (`majra` pub/sub, `bote`/`setu` endpoint) instead — determines if the samvada extension is in scope.
- **Versioning:** the Cyrius port starts at `0.1.0`; the Rust line was calendar-versioned
  (`2026.3.22`). Confirm the new version scheme.
