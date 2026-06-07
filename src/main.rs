mod bar;
mod cli;
mod color;
mod format;
mod input;

fn main() {
    let _args = crate::cli::Cli::parse_args();
}
