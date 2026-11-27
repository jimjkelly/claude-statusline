//! Screen-lock and power state of the machine running Claude Code, and the
//! work tier derived from it.
//!
//! Only macOS can be read; elsewhere [`read`] returns `None`. Everything
//! outside the `macos` module is plain data and pure functions.

use serde::Serialize;

#[cfg(target_os = "macos")]
mod macos;

/// Battery current. Negative while the battery discharges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Milliamps(pub i32);

/// Battery terminal voltage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Millivolts(pub u32);

/// State of charge, 0 to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Percent(pub u8);

/// Where the machine draws power from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Ac,
    Battery,
}

/// Whether someone is at the machine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Presence {
    pub locked: bool,
    pub idle_seconds: f64,
}

/// Battery and charger readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Power {
    pub source: Source,
    pub percent: Percent,
    pub charging: bool,
    pub current: Milliamps,
    pub voltage: Millivolts,
    pub adapter_watts: Option<u32>,
}

/// How hard Claude should work, given the power situation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Full,
    Conserve,
    Minimal,
}

/// Averaged current below this counts as draining, so a burst on a charger
/// that otherwise keeps up does not flip the tier.
const DRAIN_THRESHOLD: Milliamps = Milliamps(-100);
/// On battery, conserve at or below this charge.
const CONSERVE_AT_OR_BELOW: Percent = Percent(50);
/// Below this charge, heavy work stops whatever the source.
const MINIMAL_BELOW: Percent = Percent(20);

impl Power {
    /// Power into (positive) or out of (negative) the battery.
    #[must_use]
    pub fn battery_watts(&self) -> f64 {
        f64::from(self.current.0) * f64::from(self.voltage.0) / 1_000_000.0
    }

    fn draining(&self) -> bool {
        self.current.0 < DRAIN_THRESHOLD.0
    }
}

impl Tier {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Conserve => "conserve",
            Self::Minimal => "minimal",
        }
    }
}

/// One reading of the machine. Either half is `None` when it cannot be read:
/// no GUI session for presence, no battery for power.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Snapshot {
    pub presence: Option<Presence>,
    pub power: Option<Power>,
}

impl Snapshot {
    #[must_use]
    pub fn tier(&self) -> Tier {
        let Some(power) = self.power else {
            return Tier::Full;
        };
        // On AC the battery matters only when the charger is losing; on
        // battery it always drains, so the charge left decides.
        let draining = match power.source {
            Source::Ac => power.draining(),
            Source::Battery => true,
        };
        let comfortable = match power.source {
            Source::Ac => false,
            Source::Battery => power.percent > CONSERVE_AT_OR_BELOW,
        };
        if !draining || comfortable {
            Tier::Full
        } else if power.percent < MINIMAL_BELOW {
            Tier::Minimal
        } else {
            Tier::Conserve
        }
    }

    #[must_use]
    pub fn reason(&self) -> String {
        let Some(power) = self.power else {
            return "no battery".to_string();
        };
        let outflow = -power.battery_watts();
        match (power.source, power.adapter_watts) {
            (Source::Ac, _) if !power.draining() => "on AC, battery holding".to_string(),
            (Source::Ac, Some(w)) => format!("on AC ({w} W) but battery draining {outflow:.1} W"),
            (Source::Ac, None) => format!("on AC but battery draining {outflow:.1} W"),
            (Source::Battery, _) => {
                format!("on battery at {}%, drawing {outflow:.1} W", power.percent.0)
            }
        }
    }
}

/// The JSON printed by `claude-statusline state`.
#[derive(Debug, Serialize)]
pub struct Report {
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<Presence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<Tier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The `power` object in [`Report`], with units in the field names.
#[derive(Debug, Serialize)]
pub struct PowerReport {
    pub source: Source,
    pub battery_percent: Percent,
    pub charging: bool,
    pub battery_current_ma: Milliamps,
    pub battery_voltage_mv: Millivolts,
    pub battery_watts: f64,
    pub adapter_watts: Option<u32>,
}

impl Report {
    #[must_use]
    pub fn new(snapshot: Option<Snapshot>) -> Self {
        let Some(snapshot) = snapshot else {
            return Self {
                supported: false,
                presence: None,
                power: None,
                tier: None,
                reason: None,
            };
        };
        Self {
            supported: true,
            presence: snapshot.presence,
            power: snapshot.power.map(|p| PowerReport {
                source: p.source,
                battery_percent: p.percent,
                charging: p.charging,
                battery_current_ma: p.current,
                battery_voltage_mv: p.voltage,
                battery_watts: (p.battery_watts() * 100.0).round() / 100.0,
                adapter_watts: p.adapter_watts,
            }),
            tier: Some(snapshot.tier()),
            reason: Some(snapshot.reason()),
        }
    }
}

/// Read the machine's state, or `None` where reading is unsupported.
#[must_use]
pub fn read() -> Option<Snapshot> {
    #[cfg(target_os = "macos")]
    {
        Some(macos::read())
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ac(current: i32, percent: u8) -> Snapshot {
        Snapshot {
            presence: None,
            power: Some(Power {
                source: Source::Ac,
                percent: Percent(percent),
                charging: false,
                current: Milliamps(current),
                voltage: Millivolts(12_700),
                adapter_watts: Some(30),
            }),
        }
    }

    fn battery(percent: u8) -> Snapshot {
        Snapshot {
            presence: None,
            power: Some(Power {
                source: Source::Battery,
                percent: Percent(percent),
                charging: false,
                current: Milliamps(-770),
                voltage: Millivolts(12_700),
                adapter_watts: None,
            }),
        }
    }

    #[test]
    fn no_battery_is_full() {
        assert_eq!(Snapshot::default().tier(), Tier::Full);
        assert_eq!(Snapshot::default().reason(), "no battery");
    }

    #[test]
    fn ac_holding_charge_is_full() {
        assert_eq!(ac(0, 30).tier(), Tier::Full);
        assert_eq!(ac(1_500, 30).tier(), Tier::Full);
    }

    #[test]
    fn ac_draw_inside_threshold_is_full() {
        assert_eq!(ac(-99, 80).tier(), Tier::Full);
        assert_eq!(ac(-100, 80).tier(), Tier::Full);
    }

    #[test]
    fn ac_draining_is_conserve() {
        assert_eq!(ac(-101, 80).tier(), Tier::Conserve);
        assert_eq!(ac(-480, 20).tier(), Tier::Conserve);
    }

    #[test]
    fn ac_draining_below_twenty_is_minimal() {
        assert_eq!(ac(-480, 19).tier(), Tier::Minimal);
    }

    #[test]
    fn battery_bands() {
        assert_eq!(battery(51).tier(), Tier::Full);
        assert_eq!(battery(50).tier(), Tier::Conserve);
        assert_eq!(battery(20).tier(), Tier::Conserve);
        assert_eq!(battery(19).tier(), Tier::Minimal);
    }

    #[test]
    fn battery_watts_is_signed() {
        let Some(power) = ac(-480, 78).power else {
            return;
        };
        assert!((power.battery_watts() - -6.096).abs() < 1e-9);
    }

    #[test]
    fn reasons_name_the_cause() {
        assert_eq!(ac(0, 80).reason(), "on AC, battery holding");
        assert_eq!(
            ac(-480, 78).reason(),
            "on AC (30 W) but battery draining 6.1 W"
        );
        assert_eq!(battery(78).reason(), "on battery at 78%, drawing 9.8 W");
    }

    #[test]
    #[expect(clippy::unwrap_used, reason = "serializing a plain struct cannot fail")]
    fn unsupported_report_is_minimal_json() {
        let json = serde_json::to_string(&Report::new(None)).unwrap();
        assert_eq!(json, r#"{"supported":false}"#);
    }

    #[test]
    #[expect(clippy::unwrap_used, reason = "serializing a plain struct cannot fail")]
    fn supported_report_carries_tier_and_rounded_watts() {
        let json = serde_json::to_value(Report::new(Some(ac(-480, 78)))).unwrap();
        assert_eq!(json["supported"], true);
        assert_eq!(json["tier"], "conserve");
        assert_eq!(json["reason"], "on AC (30 W) but battery draining 6.1 W");
        assert_eq!(json["power"]["source"], "ac");
        assert_eq!(json["power"]["battery_percent"], 78);
        assert_eq!(json["power"]["battery_current_ma"], -480);
        assert_eq!(json["power"]["battery_watts"], -6.1);
        assert_eq!(json["power"]["adapter_watts"], 30);
        assert!(json.get("presence").is_none());
    }
}
