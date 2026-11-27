use std::fmt::Write;
use std::path::Path;

use unicode_width::UnicodeWidthChar;

use crate::bar;
use crate::color::{self, Color};
use crate::format;
use crate::git::MAX_SCANNED_COMMITS;
use crate::host::{Snapshot, Source, Tier, Watts};
use crate::input::Input;
use crate::pace;
use crate::theme::Theme;

/// Facts about the machine and checkout that do not come from Claude Code's
/// stdin.
#[derive(Debug, Clone, Copy, Default)]
pub struct Local<'a> {
    pub branch: Option<&'a str>,
    pub unsigned_commits: usize,
    pub host: Option<Snapshot>,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub theme: Theme,
    pub color_enabled: bool,
    pub now_epoch: i64,
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

fn line_one_with_bar_width(input: &Input, opts: RenderOptions, bar_width: usize) -> String {
    let mut out = String::new();
    let theme = opts.theme;

    if let Some(w) = input.rate_limits.five_hour {
        let bar = bar::render(w.used_percentage, bar_width);
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
        let bar = bar::render(w.used_percentage, bar_width);
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

/// Render line 1: sess bar, week bar, model.
#[must_use]
pub fn line_one(input: &Input, opts: RenderOptions) -> String {
    line_one_with_bar_width(input, opts, 12)
}

fn line_one_short(input: &Input, opts: RenderOptions) -> String {
    line_one_with_bar_width(input, opts, 4)
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

fn duration_segment(input: &Input, opts: RenderOptions, out: &mut String) {
    if input.cost.total_duration_ms == 0 {
        return;
    }
    out.push_str(opts.theme.duration());
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

/// Shown on battery or whenever the tier is not `full`; hidden on AC at
/// `full`, where there is nothing to act on.
fn power_segment(host: Option<Snapshot>, opts: RenderOptions, out: &mut String) {
    let Some(snapshot) = host else { return };
    let Some(power) = snapshot.power else { return };
    let tier = snapshot.tier();
    if power.source == Source::Ac && tier == Tier::Full {
        return;
    }
    let color = match tier {
        Tier::Full => Color::Default,
        Tier::Conserve => Color::Yellow,
        Tier::Minimal => Color::Red,
    };
    out.push_str(opts.theme.power());
    paint(out, color, opts.color_enabled, tier.as_str());
    match (power.source, power.adapter) {
        (Source::Ac, Some(Watts(w))) => {
            let _ = write!(out, " {w}W");
        }
        (Source::Ac, None) => out.push_str(" AC"),
        (Source::Battery, _) => out.push_str(" bat"),
    }
    let _ = write!(out, " {:+.1}W {}%", power.battery_watts(), power.percent.0);
}

struct Segment {
    kind: &'static str,
    body: String,
}

fn push_if_nonempty(segs: &mut Vec<Segment>, kind: &'static str, body: String) {
    if !body.is_empty() {
        segs.push(Segment { kind, body });
    }
}

fn line_two_segments(input: &Input, host: Option<Snapshot>, opts: RenderOptions) -> Vec<Segment> {
    let mut segs: Vec<Segment> = Vec::new();

    let mut tmp = String::new();
    power_segment(host, opts, &mut tmp);
    push_if_nonempty(&mut segs, "power", std::mem::take(&mut tmp));

    ctx_segment(input, opts, &mut tmp);
    push_if_nonempty(&mut segs, "ctx", std::mem::take(&mut tmp));

    pace_segment(input, opts, &mut tmp);
    push_if_nonempty(&mut segs, "pace", std::mem::take(&mut tmp));

    cost_segment(input, opts, &mut tmp);
    push_if_nonempty(&mut segs, "cost", std::mem::take(&mut tmp));

    duration_segment(input, opts, &mut tmp);
    push_if_nonempty(&mut segs, "duration", std::mem::take(&mut tmp));

    lines_segment(input, &mut tmp);
    push_if_nonempty(&mut segs, "lines", std::mem::take(&mut tmp));

    pr_segment(input, &mut tmp);
    push_if_nonempty(&mut segs, "pr", std::mem::take(&mut tmp));

    segs
}

fn join_segments(segs: &[Segment]) -> String {
    let bodies: Vec<&str> = segs.iter().map(|s| s.body.as_str()).collect();
    bodies.join(" · ")
}

/// Render line 2: power, ctx bar, pace, cost, duration, lines, pr.
#[must_use]
pub fn line_two(input: &Input, host: Option<Snapshot>, opts: RenderOptions) -> String {
    join_segments(&line_two_segments(input, host, opts))
}

/// Render line 3: working directory, git branch, and unsigned commits.
#[must_use]
pub fn line_three(input: &Input, local: Local<'_>, opts: RenderOptions) -> String {
    let mut parts: Vec<String> = Vec::new();

    let dir_name = Path::new(&input.workspace.current_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !dir_name.is_empty() {
        let mut s = String::new();
        s.push_str(opts.theme.dir());
        s.push_str(dir_name);
        parts.push(s);
    }

    if let Some(b) = local.branch {
        if !b.is_empty() {
            let mut s = String::new();
            s.push_str(opts.theme.branch());
            s.push_str(b);
            parts.push(s);
        }
    }

    if local.unsigned_commits > 0 {
        let mut s = String::new();
        let body = if local.unsigned_commits >= MAX_SCANNED_COMMITS {
            format!("{MAX_SCANNED_COMMITS}+ unsigned")
        } else {
            format!("{} unsigned", local.unsigned_commits)
        };
        paint(&mut s, Color::Yellow, opts.color_enabled, &body);
        parts.push(s);
    }

    parts.join(" · ")
}

/// Count display columns, skipping ANSI SGR escape sequences. Uses East Asian
/// width so a CJK directory name costs the two columns it actually occupies.
fn visible_width(s: &str) -> usize {
    let mut count = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for x in chars.by_ref() {
                if x == 'm' {
                    break;
                }
            }
            continue;
        }
        count += c.width().unwrap_or(0);
    }
    count
}

const DROP_ORDER: &[&str] = &["lines", "duration", "pr", "cost", "pace", "ctx", "power"];

/// Render the full three-line statusline with responsive collapse.
///
/// Drops low-priority segments from line 2 when the joined width exceeds
/// `opts.columns`. If line 1 still overflows, shrinks the session/weekly
/// bars from 12 to 4 cells. Line 3 (dir + branch) is never collapsed.
#[must_use]
pub fn render(input: &Input, local: Local<'_>, opts: RenderOptions) -> String {
    let mut segs = line_two_segments(input, local.host, opts);
    let mut line2 = join_segments(&segs);
    let max = opts.columns;

    let mut drop_iter = DROP_ORDER.iter();
    while visible_width(&line2) > max {
        let Some(kind) = drop_iter.next() else { break };
        if let Some(idx) = segs.iter().position(|s| &s.kind == kind) {
            segs.remove(idx);
            line2 = join_segments(&segs);
        }
    }

    let line1 = line_one(input, opts);
    let line1_final = if visible_width(&line1) > max {
        line_one_short(input, opts)
    } else {
        line1
    };

    let line3 = line_three(input, local, opts);
    let mut lines: Vec<&str> = Vec::with_capacity(3);
    lines.push(&line1_final);
    if !line2.is_empty() {
        lines.push(&line2);
    }
    if !line3.is_empty() {
        lines.push(&line3);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Milliamps, Millivolts, Percent, Power};

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn fixture() -> Input {
        let bytes = include_bytes!("../tests/fixtures/full.json");
        Input::from_reader(&bytes[..]).unwrap()
    }

    fn on_main() -> Local<'static> {
        Local {
            branch: Some("main"),
            ..Local::default()
        }
    }

    fn host(source: Source, current: i32, percent: u8, adapter: Option<u32>) -> Snapshot {
        Snapshot {
            presence: None,
            power: Some(Power {
                source,
                percent: Percent(percent),
                charging: false,
                current: Milliamps(current),
                voltage: Millivolts(12_700),
                adapter: adapter.map(Watts),
            }),
        }
    }

    fn render_wide(local: Local<'_>, color: bool) -> String {
        render(
            &fixture(),
            local,
            RenderOptions::new(Theme::Plain, color, NOW_EPOCH, 200),
        )
    }

    #[test]
    fn power_hidden_on_ac_at_full() {
        let local = Local {
            host: Some(host(Source::Ac, 0, 100, Some(140))),
            ..on_main()
        };
        assert!(!render_wide(local, false).contains("Pwr"));
    }

    #[test]
    fn power_hidden_without_a_battery() {
        let local = Local {
            host: Some(Snapshot::default()),
            ..on_main()
        };
        assert!(!render_wide(local, false).contains("Pwr"));
    }

    #[test]
    fn power_shown_on_battery_even_at_full() {
        let local = Local {
            host: Some(host(Source::Battery, -770, 82, None)),
            ..on_main()
        };
        assert!(render_wide(local, false).contains("Pwr full bat -9.8W 82%"));
    }

    #[test]
    fn power_shown_on_draining_charger() {
        let local = Local {
            host: Some(host(Source::Ac, -480, 78, Some(30))),
            ..on_main()
        };
        let line2 = render_wide(local, false).lines().nth(1).map(str::to_owned);
        assert_eq!(
            line2.as_deref().and_then(|l| l.split(" · ").next()),
            Some("Pwr conserve 30W -6.1W 78%")
        );
    }

    #[test]
    fn minimal_tier_is_red() {
        let local = Local {
            host: Some(host(Source::Battery, -770, 15, None)),
            ..on_main()
        };
        assert!(render_wide(local, true).contains("\u{1b}[31mminimal\u{1b}[0m"));
    }

    #[test]
    fn power_outlasts_other_segments_when_narrow() {
        let local = Local {
            host: Some(host(Source::Ac, -480, 78, Some(30))),
            ..on_main()
        };
        let out = render(
            &fixture(),
            local,
            RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 40),
        );
        assert!(out.contains("Pwr conserve"));
        assert!(!out.contains("Ctx ["));
    }

    #[test]
    fn unsigned_commits_follow_the_branch() {
        let local = Local {
            unsigned_commits: 3,
            ..on_main()
        };
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        assert!(line_three(&fixture(), local, opts).ends_with("main · 3 unsigned"));
    }

    #[test]
    fn unsigned_count_at_the_scan_cap_reads_as_a_floor() {
        let local = Local {
            unsigned_commits: MAX_SCANNED_COMMITS,
            ..on_main()
        };
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        assert!(line_three(&fixture(), local, opts).ends_with("main · 200+ unsigned"));
    }

    #[test]
    fn no_unsigned_segment_at_zero() {
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        assert!(!line_three(&fixture(), on_main(), opts).contains("unsigned"));
    }

    // resets_at 1_749_250_000; pick now 1 hour earlier.
    const NOW_EPOCH: i64 = 1_749_246_400;

    #[test]
    fn visible_width_counts_wide_chars_as_two_columns() {
        assert_eq!(visible_width("日本語"), 6);
        assert_eq!(visible_width("ascii"), 5);
    }

    #[test]
    fn wide_dir_name_triggers_collapse() {
        let mut input = fixture();
        // 30 CJK chars occupy 60 columns; a naive char count would see 30.
        input.workspace.current_dir = "/tmp/".to_string() + &"日".repeat(30);
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 60);
        let out = render(&input, on_main(), opts);
        let line3 = out.lines().last().unwrap_or_default();
        assert_eq!(visible_width(line3), 60 + visible_width(" · main"));
    }

    #[test]
    fn line_one_plain_no_color() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        let s = line_one(&input, opts);
        assert!(s.contains("Sess ["));
        assert!(s.contains("Week ["));
        assert!(s.contains("Opus 4.7"));
        assert!(!s.contains("\u{1b}["));
    }

    #[test]
    fn line_one_color_emits_ansi() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, true, NOW_EPOCH, 200);
        let s = line_one(&input, opts);
        assert!(s.contains("\u{1b}["));
    }

    #[test]
    fn line_two_full_fixture() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        let s = line_two(&input, None, opts);
        assert!(s.contains("Ctx ["));
        assert!(s.contains("Pace ["));
        assert!(s.contains("$1.23"));
        assert!(s.contains("12m"));
        assert!(s.contains("+24/-7"));
        assert!(s.contains("#42 approved"));
        // Dir and branch live on line 3 now.
        assert!(!s.contains("claude-statusline"));
        assert!(!s.contains("main"));
    }

    #[test]
    fn line_three_full_fixture() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        let s = line_three(&input, on_main(), opts);
        assert!(s.contains("claude-statusline"));
        assert!(s.contains("main"));
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

    #[test]
    fn visible_width_strips_ansi() {
        assert_eq!(visible_width("abc"), 3);
        assert_eq!(visible_width("\u{1b}[32mabc\u{1b}[0m"), 3);
        assert_eq!(visible_width("a\u{1b}[2mb\u{1b}[0mc"), 3);
    }

    #[test]
    fn wide_terminal_keeps_everything() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 200);
        let out = render(&input, on_main(), opts);
        assert!(out.contains("+24/-7"));
        assert!(out.contains("12m"));
        assert!(out.contains("Pace ["));
        let lines: Vec<_> = out.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[2].contains("claude-statusline"));
        assert!(lines[2].contains("main"));
    }

    #[test]
    fn narrow_terminal_drops_lowest_priority_first() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 70);
        let out = render(&input, on_main(), opts);
        assert!(!out.contains("+24/-7"));
        // Line 1's Sess and Week always survive.
        assert!(out.contains("Sess "));
        assert!(out.contains("Week "));
    }

    #[test]
    fn very_narrow_terminal_shortens_line_one_bars() {
        let input = fixture();
        let opts = RenderOptions::new(Theme::Plain, false, NOW_EPOCH, 40);
        let out = render(&input, on_main(), opts);
        assert!(out.contains("Sess "));
        assert!(out.contains("Week "));
    }
}
