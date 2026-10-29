"""Regenerates the built-in word lists in words/ from wordfreq (data: CC-BY-SA 4.0).

Run from the repo root (needs Nix, or `pip install wordfreq==3.0.2`):
    nix shell --impure --expr 'with import (builtins.getFlake "nixpkgs") {}; python3.withPackages (p: [p.wordfreq])' -c python3 scripts/wordlists.py
"""

from wordfreq import top_n_list

LANGS = {"english": "en", "french": "fr"}
SIZES = {"": 200, "_1k": 1000, "_10k": 10000}
# Single letters that are real words; others ("l", "d", "s") are elision/contraction leftovers.
LETTERS = {"en": {"a"}, "fr": {"a", "à", "y"}}
# Profanity and slurs: a typing test shouldn't show them. Extend when you spot one.
BLOCKED = set("""
    fuck fucking fucked fucker fuckin motherfucker shit shitty bullshit bitch bitches dick dicks cock
    pussy cunt asshole ass bastard damn whore slut slutty nigga nigger niggas fag faggot retard retarded
    porn porno sex sexy horny tits boobs penis vagina dildo cum rape raped
    merde putain pute putes salope salopes connard connards connasse con cons conne enculé enculer
    bite chier chiant nique niquer pd pédé tapette nègre bordel cul couille couilles foutre branleur
    pétasse bouffon débile
""".split())


def words(code, n):
    out = []
    for w in top_n_list(code, n * 2):
        if w.isalpha() and w not in BLOCKED and (len(w) > 1 or w in LETTERS[code]):
            out.append(w)
            if len(out) == n:
                return out
    raise SystemExit(f"{code}: only {len(out)} words, wanted {n}")


for name, code in LANGS.items():
    for suffix, n in SIZES.items():
        with open(f"words/{name}{suffix}.txt", "w", encoding="utf-8") as f:
            f.write("\n".join(words(code, n)) + "\n")
