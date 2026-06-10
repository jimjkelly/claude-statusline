#![cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by main wiring in Task 13")
)]

use std::fmt::Write;
use std::path::Path;

use crate::bar;
use crate::color::{self, Color};
use crate::format;
use crate::input::Input;
use crate::pace;
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

fn ctx_segment(input: &Input, opts: RenderOptions, out: &mut String) {
    let Some(pct) = input.context_window.used_percentage else {
        return;
    };
    let pct_f = f64::from(pct);
    let bar = bar::render(pct_f, 12);
    let color = color::threshold(pct_f, 60.0, 85.0);
    let _ = write!(out, "{} [", opts.theme.ctx());
    paint(out, color, opts.color_enabled, &bar);
    let _ = write!(out, "] {pct}%");
}

fn pace_segment(input: &Input, opts: RenderOptions, out: &mut String) {
    let Some(week) = input.rate_limits.seven_day else {
        return;
    };
    let Some(pace) = pace::compute(week.used_percentage, week.resets_at, opts.now_epoch) else {
        return;
    };
    let pendulum = pace::render(pace, week.used_percentage, 15);
    let color = if pace.delta_pct < 0.0 {
        Color::Green
    } else if pace.delta_pct < 10.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let _ = write!(out, "{} [", opts.theme.pace());
    paint(out, color, opts.color_enabled, &pendulum);
    let _ = write!(out, "] {:+.0}%", pace.delta_pct);
}

fn cost_segment(input: &Input, opts: RenderOptions, out: &mut String) {
    let amount = input.cost.total_cost_usd;
    if amount <= 0.0 {
        return;
    }
    let color = if amount > 50.0 {
        Color::Red
    } else if amount > 10.0 {
        Color::Yellow
    } else {
        Color::Default
    };
    paint(out, color, opts.color_enabled, &format::cost_usd(amount));
}

fn duration_segment(input: &Input, out: &mut String) {
    if input.cost.total_duration_ms == 0 {
        return;
    }
    out.push_str(&format::duration_short(input.cost.total_duration_ms));
}

fn lines_segment(input: &Input, out: &mut String) {
    if input.cost.total_lines_added == 0 && input.cost.total_lines_removed == 0 {
        return;
    }
    out.push_str(&format::lines(
        input.cost.total_lines_added,
        input.cost.total_lines_removed,
    ));
}

fn pr_segment(input: &Input, out: &mut String) {
    let Some(pr) = input.pr.as_ref() else {
        return;
    };
    let _ = write!(out, "#{} {}", pr.number, pr.review_state);
}

/// Render line 2: dir, branch, ctx bar, pace, cost, duration, lines, pr.
#[must_use]
pub fn line_two(input: &Input, branch: Option<&str>, opts: RenderOptions) -> String {
    let mut segs: Vec<String> = Vec::new();

    let dir_name = Path::new(&input.workspace.current_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !dir_name.is_empty() {
        let mut s = String::new();
        s.push_str(opts.theme.dir());
        s.push_str(dir_name);
        segs.push(s);
    }

    if let Some(b) = branch {
        if !b.is_empty() {
            let mut s = String::new();
            s.push_str(opts.theme.branch());
            s.push_str(b);
            segs.push(s);
        }
    }

    let mut tmp = String::new();
    ctx_segment(input, opts, &mut tmp);
    if !tmp.is_empty() {
        segs.push(std::mem::take(&mut tmp));
    }

    pace_segment(input, opts, &mut tmp);
    if !tmp.is_empty() {
        segs.push(std::mem::take(&mut tmp));
    }

    cost_segment(input, opts, &mut tmp);
    if !tmp.is_empty() {
        segs.push(std::mem::take(&mut tmp));
    }

    duration_segment(input, &mut tmp);
    if !tmp.is_empty() {
        segs.push(std::mem::take(&mut tmp));
    }

    lines_segment(input, &mut tmp);
    if !tmp.is_empty() {
        segs.push(std::mem::take(&mut tmp));
    }

    pr_segment(input, &mut tmp);
    if !tmp.is_empty() {
        segs.push(std::mem::take(&mut tmp));
    }

    segs.join(" · ")
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

    #[test]
    fn line_two_full_fixture() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, 1_749_246_400, 200);
        let s = line_two(&input, Some("main"), opts);
        assert!(s.contains("claude-prompt"));
        assert!(s.contains("main"));
        assert!(s.contains("Ctx ["));
        assert!(s.contains("Pace ["));
        assert!(s.contains("$1.23"));
        assert!(s.contains("12m"));
        assert!(s.contains("+24/-7"));
        assert!(s.contains("#42 approved"));
    }

    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn line_two_omits_missing_segments() {
        let bytes = include_bytes!("../tests/fixtures/minimal.json");
        let input = Input::from_reader(&bytes[..]).unwrap();
        let opts = RenderOptions::new(Theme::Plain, false, 0, 200);
        let s = line_two(&input, None, opts);
        assert!(!s.contains("Ctx"));
        assert!(!s.contains("Pace"));
        assert!(!s.contains('$'));
        assert!(!s.contains('#'));
    }
}
