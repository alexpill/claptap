# README demo. Regenerate from the repo root with:
#   cargo build --release && vhs docs/demo.tape
# CLAPTAP_SEED fixes the words so the typed text below matches; temp XDG dirs keep your
# history and word lists out of it. If the word lists change, re-capture the 25 words for seed 7.

Output docs/demo.gif

Set Shell bash
Set FontSize 18
Set Width 1200
Set Height 760
Set Padding 16
Set TypingSpeed 105ms

Hide
Type "export PATH=$PWD/target/release:$PATH CLAPTAP_SEED=7 XDG_DATA_HOME=$(mktemp -d) XDG_CONFIG_HOME=$(mktemp -d) && clear"
Enter
Show

Type "claptap --words 25 --hands"
Sleep 400ms
Enter
Sleep 1.5s

# A typo, wiped with ctrl+w.
Type "she with an grwat"
Sleep 500ms
Ctrl+W
Sleep 300ms
Type "great right many there am keep made if come who well my after used where say into a have are people only"

# Results screen with the per-second graph.
Sleep 5s
Escape
Sleep 800ms
Type "q"
Sleep 300ms
