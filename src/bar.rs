/// Render a Unicode block bar of `width` characters representing `percent`
/// in 0..=100. Uses U+2588 FULL BLOCK for filled cells, U+2591 LIGHT SHADE
/// for empty. Uses standard half-away-from-zero rounding (`f64::round`).
#[must_use]
pub fn render(percent: f64, width: usize) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "filled cell count is bounded by terminal width (well under f64 mantissa range)"
    )]
    let filled = (clamped / 100.0 * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let mut out = String::with_capacity(width * 3);
    for _ in 0..filled {
        out.push('█');
    }
    for _ in 0..empty {
        out.push('░');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_at_zero() {
        assert_eq!(render(0.0, 4), "░░░░");
    }

    #[test]
    fn full_at_one_hundred() {
        assert_eq!(render(100.0, 4), "████");
    }

    #[test]
    fn half() {
        assert_eq!(render(50.0, 4), "██░░");
    }

    #[test]
    fn rounds_correctly() {
        assert_eq!(render(12.0, 4), "░░░░");
        assert_eq!(render(13.0, 4), "█░░░");
    }

    #[test]
    fn rounds_half_away_from_zero_at_width_one() {
        assert_eq!(render(50.0, 1), "█");
    }

    #[test]
    fn clamps_above_one_hundred() {
        assert_eq!(render(120.0, 4), "████");
    }

    #[test]
    fn clamps_below_zero() {
        assert_eq!(render(-10.0, 4), "░░░░");
    }

    #[test]
    fn zero_width() {
        assert_eq!(render(50.0, 0), "");
    }
}
