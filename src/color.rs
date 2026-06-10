/// ANSI color codes used by claude-prompt. Numbered to match the 8/16-color
/// palette so they render correctly even on terminals without 256-color
/// support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Dim,
    Green,
    Yellow,
    Red,
}

impl Color {
    #[must_use]
    pub fn ansi_open(self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Dim => "\x1b[2m",
            Self::Green => "\x1b[32m",
            Self::Yellow => "\x1b[33m",
            Self::Red => "\x1b[31m",
        }
    }

    #[must_use]
    pub fn ansi_close(self) -> &'static str {
        if matches!(self, Self::Default) {
            ""
        } else {
            "\x1b[0m"
        }
    }
}

/// Map a percent to green/yellow/red using thresholds.
///   < `yellow_at`  -> Green
///   < `red_at`     -> Yellow
///   otherwise      -> Red
#[must_use]
pub fn threshold(percent: f64, yellow_at: f64, red_at: f64) -> Color {
    if percent < yellow_at {
        Color::Green
    } else if percent < red_at {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Detect whether color should be emitted given the `--no-color` flag and
/// the `NO_COLOR` env var. <https://no-color.org>
#[must_use]
pub fn enabled(no_color_flag: bool, no_color_env: Option<&str>) -> bool {
    if no_color_flag {
        return false;
    }
    !matches!(no_color_env, Some(v) if !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_percent_is_green() {
        assert_eq!(threshold(40.0, 60.0, 85.0), Color::Green);
    }

    #[test]
    fn mid_percent_is_yellow() {
        assert_eq!(threshold(70.0, 60.0, 85.0), Color::Yellow);
    }

    #[test]
    fn high_percent_is_red() {
        assert_eq!(threshold(90.0, 60.0, 85.0), Color::Red);
    }

    #[test]
    fn boundary_inclusive_to_higher() {
        assert_eq!(threshold(60.0, 60.0, 85.0), Color::Yellow);
        assert_eq!(threshold(85.0, 60.0, 85.0), Color::Red);
    }

    #[test]
    fn no_color_flag_overrides() {
        assert!(!enabled(true, None));
        assert!(!enabled(true, Some("")));
    }

    #[test]
    fn no_color_env_disables() {
        assert!(!enabled(false, Some("1")));
    }

    #[test]
    fn empty_no_color_env_does_not_disable() {
        assert!(enabled(false, Some("")));
    }

    #[test]
    fn defaults_enabled() {
        assert!(enabled(false, None));
    }
}
