# claptap

A typing test for the terminal that looks like a terminal tool, not a web page.

Dense, boxed panels and braille graphs in the spirit of `btop`, built with
[ratatui](https://ratatui.rs). Type, get your WPM, accuracy and a per-second graph, and keep a
history of every run.

> Early days: the typing test and the stratagem drill work. See [docs/TODO.md](docs/TODO.md) for the roadmap
> (stats dashboard, Helldivers-style stratagem mode, config, themes, Nix module).

## Usage

```sh
claptap                    # 30 s test, English
claptap --time 60          # 60 s test
claptap --words 25         # type 25 words
claptap --lang french      # built-in: english, french
claptap --stratagem        # stratagem drill: enter arrow codes against the clock
claptap --hands --layout azerty   # show the keyboard + hands guide: qwerty (default), azerty, qwertz, colemak, dvorak
```

### On-screen keyboard

Opt-in: `H` in normal mode or `--hands`. Below the words, an ISO keyboard in your layout lights up the next key and its finger's zone,
and names the finger to use (plus shift with the other hand for capitals, backspace when the
current word has a mistake), next to two drawn hands with the finger(s) to use filled in.
The `⌨` item in the option bar cycles the layout and lights up while the keyboard is shown.

### Stratagem mode

Inspired by Helldivers: a named stratagem and its arrow code show up, enter it as fast as you can
with the arrow keys or `hjkl`. A wrong input resets the code. Finish as many as possible before
the timer runs out; you get stratagems per minute, accuracy, average time and best streak (codes
entered without a mistake in a row), plus a graph of the time each one took.

Keys: `tab` restarts, `ctrl+c` quits, `esc` enters normal mode (orange `NORMAL` badge, the
option bar has focus): `←→`/`hl` move, `enter` picks, `H` shows/hides the keyboard and hands, `?`
opens the help, `esc` goes back to typing (`INSERT`) and `q` quits.

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

Any Rust toolchain ≥ 1.88 works, no Nix needed:

```sh
cargo install --git https://github.com/alexpill/claptap
```

## Development

Plain Rust (rustup or your distro's toolchain, ≥ 1.88):

```sh
cargo run -- --words 10
cargo test
cargo clippy --all-targets -- -D warnings
```

With Nix (optional): `nix develop` gives a shell with cargo, clippy, rustfmt and rust-analyzer.
To enter it automatically with direnv, create your own `.envrc` (it is git-ignored):

```sh
echo "use flake" > .envrc && direnv allow
```

## License

[MIT](LICENSE)
