# claude-statusline

A small Rust statusline for Claude Code. Three lines, color-coded, with
session (5-hour) and weekly (7-day) rate-limit usage front and center.

```
Sess [███░░░░░░░░░] 27% 3h17m · Week [████░░░░░░░░] 34% 4d2h · Opus 4.7
Ctx [█████░░░░░░░] 39% · Pace [░░░░░█░░|░░░░░░] -24% · $1.23 · 12m · +24/-7 · #42 approved
claude-statusline · main
```

Bar colors shift green → yellow → red as usage climbs (thresholds at 60%
and 85%). The pace pendulum on line 2 compares actual weekly usage to a
flat-budget expectation: green when under pace, yellow up to 10% over,
red beyond. Line 2 collapses from a fixed priority list when the
terminal narrows.

## Install

### Nix

```bash
nix run github:jimjkelly/claude-statusline
```

Or add the flake as an input and pull `packages.<system>.default` into
your profile or home-manager config:

```nix
{
  inputs.claude-statusline.url = "github:jimjkelly/claude-statusline";
}
```

The flake also exposes a `devShells.default` with the Rust toolchain,
and a `formatter` (nixfmt).

### Cargo

```bash
cargo build --release
```

The binary lands at `./target/release/claude-statusline`. Move it onto your
PATH or reference it by absolute path.

`git` must be on `PATH` at runtime — the branch segment shells out to it.

## Wire it up

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-statusline",
    "padding": 0,
    "refreshInterval": 30
  }
}
```

Add `--nerd` to the command if your terminal has a Nerd Font installed.

## Flags

- `--nerd` — use Nerd Font glyphs in place of text labels.
- `--no-color` — disable color (also respects the `NO_COLOR` env var).
- `--debug` — pretty-print parsed stdin JSON to stderr.

## What it shows

**Line 1** (always present, never collapses): session and weekly usage
bars with time-until-reset, plus the model name.

**Line 2** (collapses from right to left at narrow widths):
context-window percentage, pace pendulum, session cost, duration, lines
added/removed, and pull-request status when Claude Code surfaces it.

**Line 3**: working directory and git branch.

When the parsed stdin lacks a field (older Claude Code, API-plan
accounts, sessions without a PR, etc.), the corresponding segment is
silently dropped.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
