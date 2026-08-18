use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use claude_statusline::cli::Cli;
use claude_statusline::color;
use claude_statusline::git;
use claude_statusline::input::Input;
use claude_statusline::render::{render, RenderOptions};
use claude_statusline::theme::Theme;

/// Shown whenever stdin is empty or unparseable. Claude Code renders our
/// stdout verbatim, so we always emit something rather than failing.
const NO_DATA: &str = "claude-statusline (no data)";

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Claude Code captures our stdout, so `tput cols` cannot see the terminal.
/// It exports `COLUMNS` instead (Claude Code 2.1.153+); older versions leave
/// it unset and we fall back to 80.
fn columns() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80)
}

/// Dump raw stdin to stderr for troubleshooting. Never fails the run: this is
/// the one path that has to work when everything else is broken.
fn debug_dump(raw: &str) {
    let mut err = io::stderr().lock();
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(value) => {
            let pretty = serde_json::to_string_pretty(&value).unwrap_or_else(|_| raw.to_string());
            let _ = writeln!(err, "{pretty}");
        }
        Err(e) => {
            let _ = writeln!(err, "claude-statusline: stdin is not valid JSON: {e}");
            let _ = writeln!(err, "{raw}");
        }
    }
}

fn run(args: &Cli) -> anyhow::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    if args.debug {
        debug_dump(&buf);
    }

    if buf.trim().is_empty() {
        return Ok(NO_DATA.to_string());
    }

    let input: Input = match Input::from_reader(buf.as_bytes()) {
        Ok(v) => v,
        Err(e) => {
            if args.debug {
                let mut err = io::stderr().lock();
                let _ = writeln!(err, "claude-statusline: parse error: {e}");
            }
            return Ok(NO_DATA.to_string());
        }
    };

    let theme = if args.nerd { Theme::Nerd } else { Theme::Plain };
    let color_on = color::enabled(args.no_color, std::env::var("NO_COLOR").ok().as_deref());
    let cwd = PathBuf::from(&input.workspace.current_dir);
    let branch = if cwd.as_os_str().is_empty() {
        None
    } else {
        git::current_branch(&cwd)
    };
    let opts = RenderOptions::new(theme, color_on, now_epoch(), columns());

    Ok(render(&input, branch.as_deref(), opts))
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse_args();
    let output = run(&args)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{output}")?;
    Ok(())
}
