# Contributing to Jalwa

Thank you for your interest in contributing to Jalwa.

## Development Workflow

1. Fork and clone the repository
2. Create a feature branch from `main`
3. Make your changes
4. Run `make check` to validate
5. Open a pull request

## Prerequisites

- Rust stable (MSRV 1.89)
- Components: `rustfmt`, `clippy`
- PipeWire (for audio output)
- Optional: `cargo-audit`, `cargo-deny`, `cargo-tarpaulin`

## Makefile Targets

| Command | Description |
|---------|-------------|
| `make check` | fmt + clippy + test |
| `make fmt` | Check formatting |
| `make clippy` | Lint with `-D warnings` |
| `make test` | Run test suite |
| `make bench` | Run benchmarks with history |
| `make build` | Build workspace |

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `jalwa` | Binary crate — CLI, TUI, MCP server |
| `jalwa-core` | Library, playlists, queue, DB persistence |
| `jalwa-playback` | Playback engine — tarang decode + PipeWire output |
| `jalwa-gui` | Desktop GUI — egui/eframe with wgpu backend |
| `jalwa-ui` | TUI rendering — ratatui widgets |
| `jalwa-ai` | AI features — recommendations, smart playlists |

## Code Style

- `cargo fmt` — mandatory
- `cargo clippy -- -D warnings` — zero warnings
- Doc comments on all public items
- No `println!` — use `tracing` for logging

## Testing

- Unit tests colocated in modules (`#[cfg(test)] mod tests`)
- Feature-gated tests with `#[cfg(feature = "...")]`
- Target: 90%+ line coverage

## Commits

- Use conventional-style messages
- One logical change per commit

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0.
