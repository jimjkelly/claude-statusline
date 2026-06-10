# claude-prompt

A small Rust statusline for Claude Code. Two lines, color-coded, with
session (5-hour) and weekly (7-day) rate-limit usage front and center.

```
Sess [███░░░░░░░░░] 27% 3h17m · Week [████░░░░░░░░] 34% 4d2h · Opus 4.7
claude-prompt · main · Ctx [█████░░░░░░░] 39% · Pace [░░░░░█░░|░░░░░░] -24% · $1.23 · 12m · +24/-7 · #42 approved
```

Bar colors shift green → yellow → red as usage climbs (thresholds at 60%
and 85%). The pace pendulum on line 2 compares actual weekly usage to a
flat-budget expectation: green when under pace, yellow up to 10% over,
red beyond. Segments collapse from a fixed priority list when the
terminal narrows.

## Install

```bash
cargo build --release
```

The binary lands at `./target/release/claude-prompt`. Move it onto your
PATH or reference it by absolute path.

## Wire it up

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-prompt",
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

**Line 2** (collapses from right to left at narrow widths): working
directory, git branch, context-window percentage, pace pendulum, session
cost, duration, lines added/removed, and pull-request status when Claude
Code surfaces it.

When the parsed stdin lacks a field (older Claude Code, API-plan
accounts, sessions without a PR, etc.), the corresponding segment is
silently dropped.
