use claude_statusline::input::Input;
use claude_statusline::render::{render, RenderOptions};
use claude_statusline::theme::Theme;

#[expect(
    clippy::unwrap_used,
    reason = "test fixtures are valid by construction"
)]
fn run_fixture(path: &str, theme: Theme, color: bool, now: i64, cols: usize) -> String {
    let bytes = std::fs::read(path).unwrap();
    let input = Input::from_reader(&bytes[..]).unwrap();
    let opts = RenderOptions::new(theme, color, now, cols);
    render(&input, Some("main"), opts)
}

// `resets_at` in full.json is 1_749_250_000; pick a `now` 1 hour before so
// time-remaining renders as "1h0m" / "2d22h", and pace lands mid-window.
const NOW_EPOCH: i64 = 1_749_246_400;

#[test]
fn snapshot_full_plain_wide() {
    let out = run_fixture(
        "tests/fixtures/full.json",
        Theme::Plain,
        false,
        NOW_EPOCH,
        200,
    );
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_minimal_plain_wide() {
    let out = run_fixture("tests/fixtures/minimal.json", Theme::Plain, false, 0, 200);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_no_rate_limits_plain_wide() {
    let out = run_fixture(
        "tests/fixtures/no_rate_limits.json",
        Theme::Plain,
        false,
        0,
        200,
    );
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_full_plain_narrow() {
    let out = run_fixture(
        "tests/fixtures/full.json",
        Theme::Plain,
        false,
        NOW_EPOCH,
        70,
    );
    insta::assert_snapshot!(out);
}
