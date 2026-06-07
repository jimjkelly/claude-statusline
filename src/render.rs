#![cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by main wiring in Task 13")
)]

use std::fmt::Write;

use crate::bar;
use crate::color::{self, Color};
use crate::format;
use crate::input::Input;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub theme: Theme,
    pub color_enabled: bool,
    pub now_epoch: i64,
    #[expect(dead_code, reason = "consumed by responsive collapse in Task 12")]
    pub columns: usize,
}

impl RenderOptions {
    #[must_use]
    pub fn new(theme: Theme, color_enabled: bool, now_epoch: i64, columns: usize) -> Self {
        Self {
            theme,
            color_enabled,
            now_epoch,
            columns,
        }
    }
}

fn paint(out: &mut String, color: Color, enabled: bool, body: &str) {
    if enabled {
        out.push_str(color.ansi_open());
    }
    out.push_str(body);
    if enabled {
        out.push_str(color.ansi_close());
    }
}

/// Render line 1: sess bar, week bar, model.
#[must_use]
pub fn line_one(input: &Input, opts: RenderOptions) -> String {
    let mut out = String::new();
    let theme = opts.theme;

    if let Some(w) = input.rate_limits.five_hour {
        let bar = bar::render(w.used_percentage, 12);
        let color = color::threshold(w.used_percentage, 60.0, 85.0);
        let _ = write!(out, "{} [", theme.sess());
        paint(&mut out, color, opts.color_enabled, &bar);
        let remaining = format::time_remaining(w.resets_at, opts.now_epoch);
        let _ = write!(out, "] {:.0}% {}", w.used_percentage, remaining);
    }

    if let Some(w) = input.rate_limits.seven_day {
        if !out.is_empty() {
            out.push_str(" · ");
        }
        let bar = bar::render(w.used_percentage, 12);
        let color = color::threshold(w.used_percentage, 60.0, 85.0);
        let _ = write!(out, "{} [", theme.week());
        paint(&mut out, color, opts.color_enabled, &bar);
        let remaining = format::time_remaining(w.resets_at, opts.now_epoch);
        let _ = write!(out, "] {:.0}% {}", w.used_percentage, remaining);
    }

    if !input.model.display_name.is_empty() {
        if !out.is_empty() {
            out.push_str(" · ");
        }
        out.push_str(&input.model.display_name);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn fixture() -> Input {
        let bytes = include_bytes!("../tests/fixtures/full.json");
        Input::from_reader(&bytes[..]).unwrap()
    }

    #[test]
    fn line_one_plain_no_color() {
        let input = fixture();
        // resets_at 1_749_250_000; pick now 1 hour earlier
        let opts = RenderOptions::new(Theme::Plain, false, 1_749_246_400, 200);
        let s = line_one(&input, opts);
        assert!(s.contains("Sess ["));
        assert!(s.contains("Week ["));
        assert!(s.contains("Opus 4.7"));
        // No ANSI escapes when color disabled.
        assert!(!s.contains("\u{1b}["));
    }

    #[test]
    fn line_one_color_emits_ansi() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, true, 1_749_246_400, 200);
        let s = line_one(&input, opts);
        assert!(s.contains("\u{1b}["));
    }
}
