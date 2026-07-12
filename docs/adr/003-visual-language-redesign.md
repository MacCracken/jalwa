# 0003 — Visual-language redesign: deep-void palette + three selectable dark themes

**Status**: Accepted
**Date**: 2026-07-12

## Context

`rust-old/crates/jalwa-gui/src/theme.rs` defined one "AGNOS-standard" dark
palette: mid-grey surfaces (`BG_DARK` #18181C, `BG_PANEL` #202026,
`BG_WIDGET` #2C2C34), a single cyan accent (#00BCD4), and grey text. The Cyrius
port carried it forward verbatim in `src/gui/theme.cyr`, and `gui_theme.tcyr`
pinned those exact bytes as a rust-old parity test.

The **"Jalwa Visual Language" north-star** ([`docs/development/design/`](../development/design/README.md)
— `Jalwa Visual Language.dc.html` + per-view mockups) reframes the aesthetic.
*jalwa* means theophany — the manifestation of
sound as light — so the design keeps the dark-and-cyan **soul** but pushes the
base to a near-black **void** from which the accent *glows*, and proposes three
whole visual languages ("three ways to manifest"), all faithful to that soul:

- **A — Aurora Void** — luminous cyan drifting into violet; the truest evolution
  of today's identity.
- **B — Caustic Glass** — aqua→teal, crisper and higher-contrast.
- **C — Sacred Bloom** — a gold core over a warm void.

The GPU-era parts of that doc (volumetric aurora / caustic / mandala visualizers,
frosted-glass blur, 60fps bloom) are **out of scope here** — jalwa's GUI is a CPU
framebuffer rasterizer (`gui/raster.cyr`, no shader/GPU path exists yet; see the
GUI decisions in CLAUDE.md). What *is* implementable now, and what this ADR
covers, is the **color layer**: the palette and the three themes.

The port's standing rule is "match rust-old; diverge only with an ADR." Changing
the palette is a deliberate divergence, so it needs this record, and the old
parity test has to become a design-spec test.

## Decision

Replace the single ported palette with a **deep-void base palette and three
selectable dark themes**, keeping the 8-slot color contract unchanged so no view
builder has to change.

- **Palette model** — `src/gui/theme.cyr` keeps the same 8 `JLW_GUI_*` packed
  `0xRRGGBBAA` slots (BG_DARK/PANEL/WIDGET, ACCENT, ACCENT_DIM, TEXT
  PRIMARY/SECONDARY/MUTED) and the byte accessors. Those 8 are now the **active
  palette**: mutable module globals, initialized to Aurora Void.
- **Three themes** — each is a full 8-slot constant set (`JLW_AURORA_*`,
  `JLW_CAUSTIC_*`, `JLW_SACRED_*`), with the exact colors taken from the designer's
  per-view mockups in [`docs/development/design/`](../development/design/README.md).
  All sit on a near-black void (#0A0A10 / #04090C / #0B0906) with primary text a
  constant #F2F3F7; accents are Aurora cyan #3DE7FF, Caustic bright cyan #6FE6FF,
  Sacred gold #FFD76B. `TEXT_SECONDARY`/`TEXT_MUTED` are the opaque equivalents of
  the design's `rgba(242,243,247, .6 / .45)` text ramp composited over the void
  (jalwa draws opaque glyphs — no per-glyph alpha), which keeps muted text ≥3:1 on
  every surface.
- **Switching** — `jlw_gui_theme_apply(id)` reseats the 8 active globals from the
  chosen set (out-of-range → Aurora); because every one of the ~50 call sites
  reads the same `JLW_GUI_*` symbols, switching costs nothing downstream.
  `jlw_gui_theme_cycle()` advances with wrap.
- **Selection surface** — in the running GUI, **`t`** cycles the theme
  (`draw.cyr` `key_to_action` → `control.cyr` `apply_action` →
  `jlw_gui_theme_cycle`). At launch, **`$JALWA_THEME`** (`aurora`|`caustic`|
  `sacred`) picks the starting theme; `jlw_gui_run` calls
  `jlw_gui_theme_init_from_env()` once before the first paint.
- **Default** — Aurora Void, per the doc ("the truest evolution of today's
  identity").
- **Test** — `gui_theme.tcyr` is rewritten from a parity test into a **design-spec
  test**: it pins the Aurora default, each theme's anchor colors, and the
  apply/cycle/name/from-env machinery.

## Consequences

- **Positive** — jalwa gains the redesigned look (a real near-black void that lets
  the accent read as light) plus three switchable dark themes, with zero churn at
  the draw/raster/view call sites and no new runtime cost (theme switch is 8
  global stores). Selection is both interactive (`t`) and scriptable
  (`JALWA_THEME`). The three palettes are named constants — self-documenting and
  individually testable.
- **Negative** — the palette **diverges from the rust-old oracle** by design;
  `gui_theme.tcyr` is no longer parity with `theme.rs`, and the old egui
  `apply(ctx)` mapping stays permanently deferred (there is no Cyrius egui).
  The present shell is still headless-unverifiable, so the *rendered* look of each
  theme can only be confirmed on a live compositor — the color *logic*, however,
  is fully unit-tested.
- **Neutral** — `jlw_gui_theme_name()` exists for a future on-screen theme label
  but nothing renders it yet. Theme choice is not persisted (env/keypress only). A
  future GPU path can layer the doc's volumetric visualizers on top of whichever
  theme is active without changing this color contract.

## Alternatives considered

- **Keep the single ported palette** — rejected: the explicit ask was to adopt the
  new visual language; staying byte-parity with `theme.rs` defeats the redesign.
- **Make each `JLW_GUI_*` a function reading a theme id** — rejected: every call
  site uses the bare symbol (`JLW_GUI_ACCENT`), not a call, so this would touch
  ~50 sites across five files for no benefit over reseating mutable globals.
- **Env-var only (no in-app key)** — rejected: cycling live with `t` is the
  headless-testable, discoverable path; env selection is the convenience layer on
  top.
- **Pick one direction (A/B/C) and ship only it** — rejected: the doc frames the
  three as a choice for the user, and the 8-slot model makes carrying all three
  nearly free.
