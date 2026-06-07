/// Format a duration in milliseconds as `3h17m`, `12m`, or `45s`. Always
/// two-component max; the smaller component is dropped when the larger is
/// hours or more.
#[must_use]
pub fn duration_short(ms: u64) -> String {
    let secs = ms / 1_000;
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    let seconds = secs % 60;
    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{seconds}s")
    }
}

/// Format remaining time until a Unix epoch timestamp. Returns `0s` if the
/// target is in the past.
#[must_use]
pub fn time_remaining(target_epoch: i64, now_epoch: i64) -> String {
    let delta = target_epoch.saturating_sub(now_epoch).max(0);
    #[expect(clippy::cast_sign_loss, reason = "delta is non-negative after max(0)")]
    let ms = (delta as u64).saturating_mul(1_000);
    duration_short(ms)
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by renderer starting in Task 10")
)]
#[must_use]
pub fn cost_usd(amount: f64) -> String {
    format!("${amount:.2}")
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by renderer starting in Task 10")
)]
#[must_use]
pub fn lines(added: u64, removed: u64) -> String {
    format!("+{added}/-{removed}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_seconds() {
        assert_eq!(duration_short(45_000), "45s");
    }

    #[test]
    fn duration_minutes() {
        assert_eq!(duration_short(12 * 60_000), "12m");
    }

    #[test]
    fn duration_hours_and_minutes() {
        assert_eq!(duration_short((3 * 3600 + 17 * 60) * 1_000), "3h17m");
    }

    #[test]
    fn duration_days_and_hours() {
        assert_eq!(duration_short((4 * 86_400 + 2 * 3600) * 1_000), "4d2h");
    }

    #[test]
    fn duration_zero() {
        assert_eq!(duration_short(0), "0s");
    }

    #[test]
    fn time_remaining_future() {
        assert_eq!(time_remaining(1_000 + 3_600, 1_000), "1h0m");
    }

    #[test]
    fn time_remaining_past() {
        assert_eq!(time_remaining(500, 1_000), "0s");
    }

    #[test]
    fn cost_formats() {
        assert_eq!(cost_usd(1.234), "$1.23");
        assert_eq!(cost_usd(0.0), "$0.00");
    }

    #[test]
    fn lines_format() {
        assert_eq!(lines(24, 7), "+24/-7");
    }
}
