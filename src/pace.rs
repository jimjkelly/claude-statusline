#![cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by renderer starting in Task 11")
)]

const SEVEN_DAYS_SECS: i64 = 7 * 24 * 60 * 60;

/// Computed pace state for the weekly window.
#[derive(Debug, Clone, Copy)]
pub struct Pace {
    /// `elapsed_fraction` in 0.0..=1.0
    pub elapsed: f64,
    /// delta percentage: used - expected
    pub delta_pct: f64,
}

/// Returns `None` when `now` is outside the 7-day window ending at
/// `resets_at` (exclusive at the start, inclusive at the end).
#[must_use]
pub fn compute(used_pct: f64, resets_at: i64, now: i64) -> Option<Pace> {
    let window_start = resets_at - SEVEN_DAYS_SECS;
    let elapsed_secs = now - window_start;
    if elapsed_secs <= 0 || elapsed_secs > SEVEN_DAYS_SECS {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "elapsed_secs is bounded by SEVEN_DAYS_SECS (well under f64 mantissa range)"
    )]
    let elapsed = elapsed_secs as f64 / SEVEN_DAYS_SECS as f64;
    let expected = elapsed * 100.0;
    let delta_pct = used_pct - expected;
    Some(Pace { elapsed, delta_pct })
}

/// Render a 15-char pendulum bar. Center pipe marks the "expected" position
/// (`elapsed * inner_width`). Filled square (`█`) marks "actual"
/// (`used / 100 * inner_width`). Empty cells use `░`.
#[must_use]
pub fn render(pace: Pace, used_pct: f64, width: usize) -> String {
    if width < 3 {
        return String::new();
    }
    let inner = width - 1; // reserve one column for the center pipe
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "positions are bounded by inner_width (small terminal column counts)"
    )]
    let expected_pos = (pace.elapsed * inner as f64).round() as usize;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "positions are bounded by inner_width (small terminal column counts)"
    )]
    let actual_pos = (used_pct.clamp(0.0, 100.0) / 100.0 * inner as f64).round() as usize;
    let mut cells: Vec<char> = (0..inner).map(|_| '░').collect();
    if let Some(cell) = cells.get_mut(actual_pos.min(inner - 1)) {
        *cell = '█';
    }
    let mut out = String::with_capacity(width * 3);
    for (i, c) in cells.iter().enumerate() {
        if i == expected_pos.min(inner) {
            out.push('|');
        }
        out.push(*c);
    }
    if expected_pos >= inner {
        out.push('|');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch_seven_days() -> i64 {
        SEVEN_DAYS_SECS
    }

    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn elapsed_half_week() {
        let resets_at = epoch_seven_days();
        let now = resets_at / 2;
        let pace = compute(50.0, resets_at, now).unwrap();
        assert!((pace.elapsed - 0.5).abs() < 0.0001);
        assert!(pace.delta_pct.abs() < 0.0001);
    }

    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn under_pace_is_negative() {
        let resets_at = epoch_seven_days();
        let now = resets_at / 2;
        let pace = compute(20.0, resets_at, now).unwrap();
        assert!(pace.delta_pct < 0.0);
    }

    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction"
    )]
    fn over_pace_is_positive() {
        let resets_at = epoch_seven_days();
        let now = resets_at / 2;
        let pace = compute(80.0, resets_at, now).unwrap();
        assert!(pace.delta_pct > 0.0);
    }

    #[test]
    fn returns_none_outside_window() {
        assert!(compute(50.0, 1_000, 10_000).is_none());
        assert!(compute(50.0, 1_000, -700_000).is_none());
    }

    #[test]
    fn render_has_center_pipe_and_marker() {
        let pace = Pace {
            elapsed: 0.5,
            delta_pct: 0.0,
        };
        let bar = render(pace, 50.0, 15);
        assert!(bar.contains('|'));
        assert!(bar.contains('█'));
        assert_eq!(bar.chars().count(), 15);
    }

    #[test]
    fn render_handles_tiny_widths() {
        let pace = Pace {
            elapsed: 0.5,
            delta_pct: 0.0,
        };
        assert_eq!(render(pace, 50.0, 0), "");
        assert_eq!(render(pace, 50.0, 2), "");
    }

    #[test]
    fn render_pipe_at_start_when_week_begins() {
        let pace = Pace {
            elapsed: 0.0,
            delta_pct: 0.0,
        };
        let bar = render(pace, 0.0, 15);
        assert!(bar.starts_with('|'));
        assert_eq!(bar.chars().count(), 15);
    }

    #[test]
    fn render_pipe_at_end_when_week_ends() {
        let pace = Pace {
            elapsed: 1.0,
            delta_pct: 0.0,
        };
        let bar = render(pace, 100.0, 15);
        assert!(bar.ends_with('|'));
        assert_eq!(bar.chars().count(), 15);
    }
}
