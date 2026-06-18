/// Theme picks between plain text labels and Nerd Font glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Plain,
    Nerd,
}

impl Theme {
    #[must_use]
    pub fn sess(self) -> &'static str {
        match self {
            Self::Plain => "Sess",
            Self::Nerd => "\u{f2f2}", // nf-fa-stopwatch (5-hour session window)
        }
    }

    #[must_use]
    pub fn week(self) -> &'static str {
        match self {
            Self::Plain => "Week",
            Self::Nerd => "\u{f073}", // nf-fa-calendar
        }
    }

    #[must_use]
    pub fn ctx(self) -> &'static str {
        match self {
            Self::Plain => "Ctx",
            Self::Nerd => "\u{f2db}", // nf-fa-microchip
        }
    }

    #[must_use]
    pub fn pace(self) -> &'static str {
        match self {
            Self::Plain => "Pace",
            Self::Nerd => "\u{f201}", // nf-fa-line_chart
        }
    }

    #[must_use]
    pub fn dir(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::Nerd => "\u{f07b} ", // nf-fa-folder
        }
    }

    #[must_use]
    pub fn branch(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::Nerd => "\u{e0a0} ", // nf-pl-branch
        }
    }

    #[must_use]
    pub fn duration(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::Nerd => "\u{f017} ", // nf-fa-clock
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_uses_words() {
        assert_eq!(Theme::Plain.sess(), "Sess");
        assert_eq!(Theme::Plain.week(), "Week");
        assert_eq!(Theme::Plain.ctx(), "Ctx");
    }

    #[test]
    fn nerd_uses_glyphs() {
        let s = Theme::Nerd.sess();
        assert_ne!(s, "Sess");
        assert!(s.chars().count() >= 1);
    }
}
