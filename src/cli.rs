use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about = "Claude Code statusline")]
pub struct Cli {
    /// Use Nerd Font glyphs in place of text labels.
    #[arg(long)]
    pub nerd: bool,

    /// Disable color output. Overrides `NO_COLOR` detection.
    #[arg(long)]
    pub no_color: bool,

    /// Pretty-print parsed stdin JSON to stderr (for troubleshooting).
    #[arg(long)]
    pub debug: bool,
}

impl Cli {
    #[must_use]
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
