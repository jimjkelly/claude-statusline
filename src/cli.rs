use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "Claude Code statusline")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

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

#[derive(Subcommand, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Print screen-lock, power and work-tier state as JSON, for hooks and scripts.
    State,
}

impl Cli {
    #[must_use]
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
