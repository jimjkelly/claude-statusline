use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use claude_prompt::cli::Cli;
use claude_prompt::color;
use claude_prompt::git;
use claude_prompt::input::Input;
use claude_prompt::render::{render, RenderOptions};
use claude_prompt::theme::Theme;

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn columns() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80)
}

fn run(args: &Cli) -> anyhow::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    if buf.trim().is_empty() {
        return Ok("claude-prompt (no data)".to_string());
    }

    let input: Input = match Input::from_reader(buf.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Ok("claude-prompt (no data)".to_string()),
    };

    if args.debug {
        let pretty =
            serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(&buf)?)?;
        let mut err = io::stderr().lock();
        writeln!(err, "{pretty}")?;
    }

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
