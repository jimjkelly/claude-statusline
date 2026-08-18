# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-18

Initial release.

### Added

- Three-line statusline for Claude Code: session (5-hour) and weekly (7-day)
  rate-limit bars with time-until-reset and model name on line 1; context
  window, weekly pace pendulum, cost, duration, lines changed, and pull-request
  state on line 2; working directory and git branch on line 3.
- Green/yellow/red thresholds at 60% and 85% for usage bars.
- Weekly pace pendulum comparing actual usage against a flat-budget
  expectation.
- Responsive collapse: line 2 sheds segments by priority and line 1 shrinks its
  bars when `COLUMNS` is tight. Requires Claude Code 2.1.153 or later, which
  exports `COLUMNS` to the statusline process.
- `--nerd` for Nerd Font glyphs, `--no-color` (also honors `NO_COLOR`), and
  `--debug` to dump parsed stdin to stderr.
- Nix flake exposing the package, a dev shell, and a formatter.
- Prebuilt release binaries for macOS (Apple Silicon and Intel), Linux (x86_64
  and ARM64), and Windows (x86_64), each with a SHA-256 checksum.

### Notes

- Input parsing degrades per field: an unrecognized shape in one part of the
  Claude Code payload drops that segment rather than blanking the statusline.
- The git branch lookup is bounded by a 500 ms timeout so a stalled repository
  cannot wedge the statusline.

[Unreleased]: https://github.com/jimjkelly/claude-statusline/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jimjkelly/claude-statusline/releases/tag/v0.1.0
