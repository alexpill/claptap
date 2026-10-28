# Roadmap

Work through phases in order. Tick items (`- [x]`) in the change that implements them.
Add items when you find work that is missing here.

## Phase 0 — Project setup

- [x] Cargo project, MIT license, README, AGENTS.md, CLAUDE.md, .gitignore, .editorconfig
- [x] Nix flake: `packages.default`, `overlays.default`, `devShells.default`, `formatter`
- [x] Nix stays optional: plain cargo works, `.envrc` is git-ignored (documented in README)
- [x] GitHub Actions: fmt, clippy, test, `nix build`, `nix flake check`
- [x] Private GitHub repo

## Phase 1 — Core typing test

- [x] Time mode (`--time`) and words mode (`--words`)
- [x] Built-in English and French word lists, user lists in `~/.config/claptap/words/<name>.txt`
- [x] Typing view: correct / wrong / extra / missed chars, cursor, backspace into a wrong previous word
- [x] Live top bar: wpm, accuracy, time left or words done
- [x] Results: wpm, raw, accuracy, consistency, chars, time
- [x] Results graph: cumulative wpm, raw per second, error markers (braille `Chart`)
- [x] Append each result to `~/.local/share/claptap/history.jsonl`
- [x] Unit tests for stats and input handling
- [ ] Ctrl+Backspace deletes the current word
- [ ] Bigger default word lists (English 1k, French 1k) as extra built-ins (`english_1k`)
- [ ] Don't count a test that was abandoned with Tab/Esc before finishing (today nothing is saved — keep it that way, add a test)

## Phase 2 — Modes and options

- [x] Config bar inside the app (`esc` focuses it, `←→`/`hl` + `enter`): mode, time 15/30/60/120, words 10/25/50/100, language (built-in + user lists)
- [ ] Punctuation and numbers toggles (as extra config bar groups)
- [ ] Quote mode (built-in quotes file per language, quote length filter)
- [ ] Zen mode (free typing, no target, ends on Esc)
- [ ] Remember last used mode/lang between runs (state file in data dir)

## Phase 2b — On-screen keyboard

- [x] ISO keyboard under the words: next key highlighted, its finger's keys tinted, finger named
- [x] Layouts: qwerty, azerty, qwertz, colemak, dvorak (`--layout`, `⌨` item in the config bar, `off`)
- [x] Shift hint for capitals (other hand's pinky), backspace hint when the word has a mistake
- [x] Finger color zones (pinky/ring/middle/index/thumb, vivid for the finger up next) and drawn hands
      beside the board with the active finger(s) filled; hands hidden when the terminal is too narrow
- [ ] Alternative keyboard styles (mockups discussed): finger columns (split, one column per finger),
      btop keycaps (box per key, ~11 lines), compact (3 letter rows, no border) — `keyboard_style` in config
- [ ] Respect terminal themes: finger colors come from the theme (phase 6), 16-color fallback
- [ ] Shifted symbols, AZERTY digits, dead keys (ê, ü...) once punctuation/numbers exist
- [ ] ANSI board (no ISO key next to left shift, wide enter) and more layouts (bépo, workman...)
- [ ] User layouts in config
- [ ] Per-finger stats: errors and speed by finger/key, heatmap on the keyboard in results
- [ ] Show the keyboard in stratagem mode with arrows/hjkl highlighted

## Phase 3 — History and stats dashboard

- [ ] `history::read_all()` tolerant of bad lines (skip + count them)
- [ ] `claptap stats` subcommand and a stats screen reachable from the app
- [ ] btop-style dashboard: WPM over sessions graph (with moving average), accuracy graph
- [ ] Breakdown table by language × mode: runs, best, average, last 10 average
- [ ] Personal bests per mode/lang, "new PB" banner on the results screen
- [ ] Filters: language, mode, period (today / 7d / 30d / all)
- [ ] Activity heatmap (runs per day), like a GitHub contribution graph
- [ ] Store per-second samples in the record (optional field) to replay graphs of past runs

## Phase 4 — Stratagem mode (Helldivers-style)

- [x] Data model: named stratagem = sequence of directions; built-in list with our own names (`src/stratagem.rs`)
- [x] Input: arrow keys and `hjkl`
- [ ] Gate `hjkl` behind `vim_mode` (default true) once config exists; optional `wasd`
- [x] Display: arrows, progress highlight, wrong input resets the code with a red flash, next stratagems queue
- [ ] Big multi-line arrow glyphs (btop-like) when the terminal is large enough
- [x] Game loop: timed round (15/30/60/120 s from the config bar or `--stratagem --time N`), streak
- [x] Results: done, per minute, accuracy, average time, best streak, graph of seconds per stratagem
- [x] Saved to history with `mode = "stratagem"` (field mapping documented on `history::Record`)
- [ ] Shown in the stats dashboard (phase 3)
- [ ] Other round types: fixed count ("10 stratagems, fastest time"), sudden death (first mistake ends it)
- [ ] User-defined stratagems in config
- [ ] Optional sound/bell on success and failure

## Phase 5 — Configuration

- [ ] `~/.config/claptap/config.toml` (add `toml` crate): default mode, lang, time/words, `vim_mode`
- [ ] Keybinding remap (restart, quit, menu, stats, directions) with validation and clear errors
- [ ] `claptap --print-default-config`
- [ ] Custom dictionaries dir setting
- [ ] Unknown keys / bad values reported, never a crash

## Phase 6 — Themes

- [ ] Theme struct replacing the constants in `ui.rs`
- [ ] A few built-in themes (default terminal colors, a dark, a light, a btop-like)
- [ ] User themes in config; 16-color fallback for basic terminals
- [ ] Never rely on color alone for errors (underline/bold too)

## Phase 7 — Packaging and release

- [ ] Home-manager module: `programs.claptap.enable`, `settings` → `config.toml`, `package`
- [ ] Release workflow: tag → GitHub release with static Linux/macOS binaries
- [ ] Publish to crates.io
- [ ] nixpkgs PR (`pkgs/by-name/cl/claptap`)
- [ ] Repo goes public; screenshots/GIF in README (vhs tape)

## Phase 8 — Polish

- [ ] Small terminal fallback (message when too small), resize during a test
- [ ] Very long words wider than the panel get wrapped instead of overflowing
- [ ] Wide/unicode characters (CJK, emoji) width handling in word wrapping
- [ ] Startup time and idle CPU (only redraw on change / once per second)
- [ ] Man page and shell completions (clap_mangen / clap_complete) in the Nix package
