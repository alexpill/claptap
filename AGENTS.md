# AGENTS.md

Instructions for AI coding agents working on this repository.

## Project

`claptap` is a terminal typing test (Monkeytype-like) that should feel like a CLI tool:
btop-style boxed panels and braille graphs, not a web clone (smassh) nor a lazygit-style UI (ttyper).
See [README.md](README.md) for usage and [docs/TODO.md](docs/TODO.md) for the roadmap.

Goals, in short: end-of-test graph, history stats by language and mode, a Helldivers-style
"stratagem" mode (arrow sequences, `hjkl` when vim mode is on, which is the default),
configurable keybindings and word lists, and a first-class Nix package.

## Status

Phases 0 and 1 are done, plus parts of 2 (in-app config bar) and 4 (stratagem drill).
Pick the next unchecked item in phase order.

## Stack

- Rust 2024 (MSRV 1.88), `ratatui` (its re-exported `crossterm` for input; no direct crossterm dep)
- `clap` (derive) for CLI, `serde` / `serde_json` for history, `directories` for XDG paths, `rand`
- Nix flake: `packages.default`, `overlays.default`, `devShells.default`. Nix is optional:
  the project must always build and be developed with plain cargo. Don't commit `.envrc`
  or anything that makes Nix or direnv required.

## Layout

- `src/main.rs` — CLI args, terminal init/restore, event loop
- `src/app.rs` — app state, modes, config bar, key handling, saving results
- `src/typing.rs` — pure test logic and stats (WPM, raw, accuracy, consistency, per-second samples); unit-tested
- `src/stratagem.rs` — stratagem drill: built-in codes, round logic and stats; unit-tested
- `src/ui.rs` — all rendering
- `src/words.rs` — built-in (`words/*.txt`, embedded) and user word lists
- `src/history.rs` — `history.jsonl` records

## Conventions

- Everything in English: code, comments, commits, docs.
- Keep logic out of `ui.rs`: anything that can be tested without a terminal goes in a pure module with unit tests.
- Follow phase order in `docs/TODO.md`; tick items (`- [x]`) in the same change that implements them,
  and add new items there when you discover work.
- Keep dependencies minimal; ratatui widgets (`Chart`, `Sparkline`, `BarChart`...) before new crates.
- Never break `history.jsonl` compatibility: new `Record` fields must be optional (`#[serde(default)]`).
- Mark known limits with a `shortcut: <limit>, <when to upgrade>` comment.

## Commands

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo run -- --words 10
nix build && nix flake check
```

To try the UI without touching your real history, set `XDG_DATA_HOME` to a temp dir.
