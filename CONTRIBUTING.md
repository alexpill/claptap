# Contributing

Thanks for helping! Bug reports, word lists, layouts and code are all welcome.
The roadmap is in [docs/TODO.md](docs/TODO.md): pick an unchecked item, or open an issue first
for anything bigger.

## Development

Any Rust toolchain ≥ 1.88 works; Nix is optional (`nix develop`).

```sh
cargo run -- --words 10
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

To try the app without touching your real history, point `XDG_DATA_HOME` at a temp dir:

```sh
XDG_DATA_HOME=$(mktemp -d) cargo run
```

`CLAPTAP_SEED=<number>` makes the word and stratagem picks reproducible, handy for bug reports.
The README GIF is recorded with [vhs](https://github.com/charmbracelet/vhs) from
[`docs/demo.tape`](docs/demo.tape) (instructions at the top of the file).

Conventions (more in [AGENTS.md](AGENTS.md)):

- Code, comments, commits and docs in English.
- Logic that can be tested without a terminal goes in a pure module with unit tests; `ui.rs` only draws.
- Few dependencies: prefer what ratatui and the standard library already offer.
- Never break `history.jsonl`: new `Record` fields must be optional (`#[serde(default)]`).
- Tick the matching item in `docs/TODO.md` in the same change.

## Adding a word list

Built-in lists are generated, never edited by hand: add the language to `LANGS` in
[`scripts/wordlists.py`](scripts/wordlists.py), extend its profanity blocklist for that language,
regenerate (the command is at the top of the script), and register the files in `src/words.rs`.
Only use sources whose license allows redistribution, and document them in
[words/README.md](words/README.md).

For a personal list, no change is needed: drop a file in `~/.config/claptap/words/<name>.txt`.

## Adding a keyboard layout

Add an entry to `LAYOUTS` in `src/keyboard.rs`: the unshifted character of each key, row by row,
and whether it is an ISO or ANSI board. The `rows_match_finger_map` test checks the row lengths.

## Releasing (maintainers)

Update `CHANGELOG.md` and the version in `Cargo.toml`, then push a `vX.Y.Z` tag: the release
workflow builds the binaries and creates the GitHub release.
