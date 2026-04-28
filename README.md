# pinyin-grep

[English](README.md) | [中文](README.zh.md)

Detect identifiers (variables, functions, types, …) that are named in
**Hanyu Pinyin** instead of English. Built on top of
[ast-grep](https://ast-grep.github.io/) — embedded as a Rust library, not
shelled out — so you get accurate AST-level matching rather than regex
guesswork.

```
$ pinyin-grep src/
src/foo.ts:4:7    high  xinXi
  └─ xin·xi  (2 syllables)
src/foo.ts:7:10   high  huoQuYongHu
  └─ huo·qu·yong·hu  (4 syllables (ambiguous))
     alt: huo=hu·o
src/foo.ts:15:7   high  GuanLiYuan
  └─ guan·li·yuan  (3 syllables (ambiguous))
     alt: guan=gu·an | yuan=yu·an
```

## Why

Mixed-language identifiers (`huoQuYongHu`, `shujuKu`, …) are extremely
common in Chinese codebases and almost always make the code harder to
read for everyone. `pinyin-grep` finds them so they can be renamed,
listed in code reviews, or fed to an LLM that proposes English
alternatives.

## Highlights

- **AST-driven**: `ast-grep-core` + `ast-grep-language` parse the file
  and capture only the AST nodes you ask for (variable names, function
  names, type names, …). No false positives from comments or strings.
- **Self-implemented Pinyin recognition**: a hardcoded Mandarin syllable
  inventory plus a DP segmenter — no external dictionary.
- **Two stable output formats**: NDJSON for tools / AI consumption,
  rich text for humans.
- **Single static binary**: no Python/Node runtime, no separate
  `ast-grep` install required.

## Install

```sh
cargo install --path .
```

This currently supports **TypeScript / TSX / Rust**. Adding more
languages is mostly a matter of adding patterns in `src/patterns.rs`.

## Usage

```sh
# Scan a directory (.gitignore-aware via the `ignore` crate)
pinyin-grep src/

# Force a language and scan stdin pipes etc.
pinyin-grep --lang ts src/

# Custom AST pattern (replaces the language's built-in pattern set)
pinyin-grep --lang ts --pattern 'const $NAME = $$$VAL' src/

# Diagnose individual identifiers without touching files
echo "huoQuYongHu\nloadFile\nxinxi" | pinyin-grep --names --min-confidence low

# NDJSON for piping into jq / AI tools
pinyin-grep src/ --format ndjson | jq '.identifier'
```

### Useful flags

| Flag | Purpose |
| --- | --- |
| `--lang <ts\|tsx\|rs>` | Force a language (otherwise inferred from the file extension). |
| `--pattern <PATTERN>` | Override the built-in pattern set. May be repeated. Requires `--lang`. |
| `--meta-var <NAME>` | Which metavariable in your pattern names the identifier. Default `NAME`. |
| `--names` | Read one identifier per line from stdin. |
| `--format <auto\|text\|ndjson>` | Output format. `auto` = NDJSON when piped, text on a TTY. |
| `--min-confidence <low\|medium\|high>` | Filter by confidence. Default `medium`. |
| `--ignore <REGEX>` | Drop identifiers whose text matches this regex. May be repeated. |
| `--show-all` | Ignore the confidence filter and show everything. |

## Built-in patterns

When you don't pass `--pattern`, the following ast-grep patterns are
applied to each file:

**TypeScript / TSX**
- `const $NAME = $$$`
- `let $NAME = $$$`
- `var $NAME = $$$`
- `function $NAME($$$) { $$$ }`
- `class $NAME { $$$ }`
- `interface $NAME { $$$ }`
- `type $NAME = $$$`
- `enum $NAME { $$$ }`

**Rust**
- `fn $NAME($$$) { $$$ }`
- `fn $NAME($$$) -> $RET { $$$ }`
- `struct $NAME { $$$ }` / `struct $NAME($$$);` / `struct $NAME;`
- `enum $NAME { $$$ }`
- `trait $NAME { $$$ }`
- `let $NAME = $$$` / `let mut $NAME = $$$`
- `const $NAME: $T = $$$` / `static $NAME: $T = $$$`
- `type $NAME = $$$`

In every case the `$NAME` metavariable is the identifier the tool
inspects.

## Output schema (NDJSON)

Each line is a self-contained JSON object. Lines and columns are
**0-based** to match ast-grep's own JSON output.

```json
{
  "file": "src/foo.ts",
  "range": {
    "start": { "line": 6, "column": 9 },
    "end":   { "line": 6, "column": 20 }
  },
  "identifier": "huoQuYongHu",
  "tokens": [
    { "text": "huo",  "syllables": [["huo"], ["hu", "o"]] },
    { "text": "qu",   "syllables": [["qu"]] },
    { "text": "yong", "syllables": [["yong"]] },
    { "text": "hu",   "syllables": [["hu"]] }
  ],
  "score": 9,
  "confidence": "high",
  "ambiguous": true
}
```

Field guarantees:

- `tokens[*].syllables` is sorted by ascending syllable count, so
  `syllables[0]` is the **preferred** (longest-match) decomposition.
- `confidence` is one of `low` / `medium` / `high`.
- `ambiguous` is `true` whenever any token has more than one valid
  segmentation (e.g. `xian` = `xian` or `xi'an`).
- In `--names` mode the `file` and `range` fields are omitted.

## How Pinyin detection works

1. **Identifier splitting.** `huoQuYongHu` is broken into
   `[huo, qu, yong, hu]` using camelCase / snake_case / kebab-case rules
   (and the special acronym rule for `URLParser` → `[url, parser]`).
2. **Syllable segmentation.** Each lowercase token is fed into a DP
   segmenter that tries every decomposition into syllables drawn from
   the hardcoded Mandarin inventory in `src/syllables.rs`.
3. **Confidence scoring.** A handful of additive/subtractive rules are
   applied:
   - `+N` for tokens that segmented successfully
   - bonuses when **every** token is valid Pinyin and the identifier
     uses an explicit word boundary
   - penalty when only some tokens are Pinyin and average syllable
     length is short (catches `userName`-style false positives)
   - small blacklist penalty for single-token identifiers that exactly
     match common English words (`me`, `men`, `pin`, …).

The defaults are tuned so that `--min-confidence medium` catches
classic Chinese-developer identifiers (`huoQu`, `xinXi`,
`huoQuYongHuXinXi`, `GuanLiYuan`, `SHU_LIANG`) while filtering out
English-heavy ones (`userName`, `loadFile`, `getUser`).

## Limitations

- Only TypeScript / TSX / Rust in v1. More languages are a small change
  in `src/lang.rs` + `src/patterns.rs`.
- Single-token identifiers are inherently noisy. `xinxi` lands at
  `low` confidence and is filtered by default — pass
  `--min-confidence low` if you want to see it.
- Tones are ignored: `huo` matches both 火 and 或. This is fine for
  identifier-level detection.
- A few syllables are also valid English words (`men`, `pan`, `die`,
  …). They incur a blacklist penalty in single-token form.

## Development

```sh
cargo build           # build
cargo test            # 40 unit + 6 integration tests
cargo clippy --all-targets -- -D warnings
cargo run -- tests/samples/   # smoke test against the fixtures
```
