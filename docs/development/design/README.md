# Design references (visual-language north-star)

Design-tool exports that capture jalwa's intended look — parked here for **long
reference until they're properly re-documented**. These are *source material*, not
a spec: they were authored in a canvas design tool (`.dc.html` = a design-canvas
document; `support.js` is its shared runtime — every `.dc.html` loads `./support.js`,
so keep them together). Open any `.dc.html` in a browser to view it.

Each per-view file renders the **three dark themes** side by side — **Aurora Void**,
**Caustic Glass**, **Sacred Bloom** — the same three now implemented in
[`src/gui/theme.cyr`](../../../src/gui/theme.cyr) (see
[ADR 0003](../../adr/003-visual-language-redesign.md)).

## Files

| File | Covers | Notes |
|---|---|---|
| `Jalwa Visual Language.dc.html` | The north-star | Void palette, typography (Instrument Serif / Hanken Grotesk / Space Mono), material & motion, the three directions (A/B/C), and the GPU-era visualizer concepts. This is the doc that drove the palette + theme port. |
| `Jalwa Now-Playing.dc.html` | Now-Playing hero | Full-bleed reactive backdrop, luminous transport — ×3 themes. |
| `Jalwa Library.dc.html` | Library view | Grid/list, search, selection — ×3 themes. |
| `Jalwa Mini-Player.dc.html` | Mini-player | Compact transport — ×3 themes. |
| `Jalwa Queue & Search.dc.html` | Queue + Search | ×3 themes. |
| `support.js` | dc-runtime | Generated bundle the `.dc.html` files depend on. Do not edit by hand. |

## Status

Tracked as the **1.2.x design→code arc** in [`../roadmap.md`](../roadmap.md).

- **Ported (v1.2.0)**: the **color layer** — deep-void palette + three selectable dark
  themes ([ADR 0003](../../adr/003-visual-language-redesign.md)) — the **Settings**
  theme picker, the **Now-Playing** hero, and the shared **sidebar + transport** chrome.
- **Ported (v1.2.1)**: the **Library** view (header eyebrow + count, list/grid selection chrome).
- **Ported (v1.2.2)**: the **Queue** view and the **Search** screen (focused query field + hints).
- **Next (1.2.3)**: Mini-player view.
- **Aspirational**: the GPU-era visualizers (aurora / caustic / mandala), frosted-glass
  blur, and 60fps bloom need a GPU path jalwa doesn't have (its GUI is a CPU-framebuffer
  rasterizer), so they stay backlogged with video.

> When these are folded into first-party design docs, replace this folder with the
> re-documented versions and drop the raw exports.
