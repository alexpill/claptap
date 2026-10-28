# claptap

A typing test for the terminal that looks like a terminal tool, not a web page.

Dense, boxed panels and braille graphs in the spirit of `btop`, built with
[ratatui](https://ratatui.rs). Type, get your WPM, accuracy and a per-second graph, and keep a
history of every run.

> Early days: the core typing test works. See [docs/TODO.md](docs/TODO.md) for the roadmap
> (stats dashboard, Helldivers-style stratagem mode, config, themes, Nix module).

## Usage

```sh
claptap                    # 30 s test, English
claptap --time 60          # 60 s test
claptap --words 25         # type 25 words
claptap --lang french      # built-in: english, french
```

Keys: `tab` restarts, `esc` quits.

Custom word lists: put whitespace-separated words in `~/.config/claptap/words/<name>.txt`
and run `claptap --lang <name>`. A file named `english.txt` or `french.txt` replaces the built-in list.

Every finished test is appended to `~/.local/share/claptap/history.jsonl`.

## Install

### Nix

```sh
nix run github:alexpill/claptap
```

Or add the flake as an input and use `packages.<system>.default` or `overlays.default`.

### Cargo

```sh
cargo install --git https://github.com/alexpill/claptap
```

## Development

```sh
nix develop          # or: direnv allow
cargo run -- --words 10
cargo test
```

## License

[MIT](LICENSE)
