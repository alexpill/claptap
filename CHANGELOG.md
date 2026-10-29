# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/).

## [Unreleased]

## [0.1.0]

First public release.

### Added

- Typing test in time (`--time`) and words (`--words`) modes, with live wpm and accuracy.
- Results screen: wpm, raw, accuracy, consistency, chars, time, and a braille graph of wpm and
  raw speed per second with error markers.
- History of every finished test in `~/.local/share/claptap/history.jsonl`.
- In-app config bar (`esc`): mode, length, word list, keyboard layout.
- Built-in word lists from wordfreq (CC-BY-SA 4.0): `english`, `french` (200 words), plus
  `_1k` and `_10k` variants; user lists in `~/.config/claptap/words/<name>.txt`.
- Stratagem drill (`--stratagem`): enter arrow codes against the clock, with arrows or `hjkl`.
- On-screen keyboard and hands guide (`H` or `--hands`): next key, finger zones, shift and
  backspace hints. Layouts: qwerty, azerty, qwertz, colemak, dvorak (ANSI or ISO per layout),
  qwerty-uk.
- `ctrl+backspace` (also `ctrl+w`, `alt+backspace`) deletes the current word.
- Vim-style normal/insert mode badge and a `?` help popup.
- "Terminal too small" message instead of a broken layout.
- Errors are shown with underline, bold or strike-through, not only color.
- Nix flake (package, overlay, dev shell); plain cargo works without Nix.

[Unreleased]: https://github.com/alexpill/claptap/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/alexpill/claptap/releases/tag/v0.1.0
