use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

/// Deserialize a field, falling back to `T::default()` when its shape does
/// not match. Claude Code's schema evolves independently of this crate, so a
/// renamed field upstream should drop one segment rather than blank the whole
/// statusline.
fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

/// Same as [`lenient`], for optional groups where "no usable value" is
/// already the meaningful fallback.
fn lenient_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Input {
    #[serde(deserialize_with = "lenient")]
    pub model: Model,
    #[serde(deserialize_with = "lenient")]
    pub workspace: Workspace,
    #[serde(deserialize_with = "lenient")]
    pub cost: Cost,
    #[serde(deserialize_with = "lenient")]
    pub context_window: ContextWindow,
    #[serde(deserialize_with = "lenient")]
    pub rate_limits: RateLimits,
    #[serde(deserialize_with = "lenient_option")]
    pub pr: Option<Pr>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Model {
    pub display_name: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Workspace {
    pub current_dir: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Cost {
    pub total_cost_usd: f64,
    pub total_duration_ms: u64,
    pub total_lines_added: u64,
    pub total_lines_removed: u64,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct ContextWindow {
    pub used_percentage: Option<u32>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct RateLimits {
    #[serde(deserialize_with = "lenient_option")]
    pub five_hour: Option<Window>,
    #[serde(deserialize_with = "lenient_option")]
    pub seven_day: Option<Window>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Window {
    pub used_percentage: f64,
    pub resets_at: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Pr {
    pub number: u64,
    pub review_state: String,
}

impl Input {
    /// Parse a Claude Code statusline JSON document from `reader`.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader fails or the bytes are not valid JSON
    /// matching the expected schema.
    pub fn from_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self> {
        let parsed: Self = serde_json::from_reader(reader)?;
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction; failure indicates a fixture or schema bug"
    )]
    #[test]
    fn parses_full_fixture() {
        let bytes = include_bytes!("../tests/fixtures/full.json");
        let input = Input::from_reader(&bytes[..]).unwrap();
        assert_eq!(input.model.display_name, "Opus 4.7");
        assert_eq!(
            input.workspace.current_dir,
            "/Users/me/dev/claude-statusline"
        );
        assert!((input.cost.total_cost_usd - 1.234).abs() < 1e-9);
        let five = input.rate_limits.five_hour.unwrap();
        assert!((five.used_percentage - 27.0).abs() < 1e-9);
        assert_eq!(five.resets_at, 1_749_250_000);
        let pr = input.pr.unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.review_state, "approved");
    }

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction; failure indicates a fixture or schema bug"
    )]
    #[test]
    fn parses_minimal_fixture() {
        let bytes = include_bytes!("../tests/fixtures/minimal.json");
        let input = Input::from_reader(&bytes[..]).unwrap();
        assert_eq!(input.model.display_name, "Sonnet 4.6");
        assert!(input.rate_limits.five_hour.is_none());
        assert!(input.rate_limits.seven_day.is_none());
        assert!(input.pr.is_none());
    }

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction; failure indicates a fixture or schema bug"
    )]
    #[test]
    fn parses_no_rate_limits_fixture() {
        let bytes = include_bytes!("../tests/fixtures/no_rate_limits.json");
        let input = Input::from_reader(&bytes[..]).unwrap();
        assert_eq!(input.context_window.used_percentage, Some(12));
        assert!(input.rate_limits.five_hour.is_none());
    }

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction; failure indicates a fixture or schema bug"
    )]
    #[test]
    fn malformed_window_drops_only_that_window() {
        let raw = br#"{
            "model": {"display_name": "Opus 4.7"},
            "rate_limits": {
                "five_hour": {"used_percentage": "not a number"},
                "seven_day": {"used_percentage": 41.2, "resets_at": 1749250000}
            }
        }"#;
        let input = Input::from_reader(&raw[..]).unwrap();
        assert!(input.rate_limits.five_hour.is_none());
        assert!(input.rate_limits.seven_day.is_some());
        assert_eq!(input.model.display_name, "Opus 4.7");
    }

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction; failure indicates a fixture or schema bug"
    )]
    #[test]
    fn malformed_group_falls_back_to_default() {
        let raw = br#"{
            "model": {"display_name": "Opus 4.7"},
            "cost": "unexpectedly a string",
            "context_window": {"used_percentage": 39}
        }"#;
        let input = Input::from_reader(&raw[..]).unwrap();
        assert!((input.cost.total_cost_usd - 0.0).abs() < f64::EPSILON);
        assert_eq!(input.model.display_name, "Opus 4.7");
        assert_eq!(input.context_window.used_percentage, Some(39));
    }

    #[expect(
        clippy::unwrap_used,
        reason = "test fixtures are valid by construction; failure indicates a fixture or schema bug"
    )]
    #[test]
    fn unknown_upstream_fields_are_ignored() {
        let raw = br#"{"model": {"display_name": "Opus 4.7", "brand_new": 1}, "future": {}}"#;
        let input = Input::from_reader(&raw[..]).unwrap();
        assert_eq!(input.model.display_name, "Opus 4.7");
    }

    #[test]
    fn non_json_still_errors() {
        assert!(Input::from_reader(&b"not json at all"[..]).is_err());
    }
}
