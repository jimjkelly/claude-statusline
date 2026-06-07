mod bar;
mod cli;
mod color;
mod format;
mod git;
mod input;
mod pace;
mod render;
mod theme;

fn main() {
    let _args = crate::cli::Cli::parse_args();
}
