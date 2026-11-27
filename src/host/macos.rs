//! Reads presence from CoreGraphics and power from the `AppleSmartBattery`
//! `IOKit` registry entry.
//!
//! The only module allowed `unsafe`. Each block wraps one FFI call or cast and
//! states the invariant it relies on.
#![expect(
    unsafe_code,
    reason = "IOKit registry lookups and CF dictionary casts have no safe binding"
)]

use std::ptr::{self, NonNull};

use objc2_core_foundation::{
    CFBoolean, CFDictionary, CFMutableDictionary, CFNumber, CFRetained, CFString, CFType,
};
use objc2_core_graphics::{
    CGEventSource, CGEventSourceStateID, CGEventType, CGSessionCopyCurrentDictionary,
};
use objc2_io_kit::{
    IOObjectRelease, IORegistryEntryCreateCFProperties, IOServiceGetMatchingService,
    IOServiceMatching,
};

use super::{Milliamps, Millivolts, Percent, Power, Presence, Seconds, Source, Watts};

type Properties = CFDictionary<CFString, CFType>;

/// `kCGAnyInputEventType`, which C defines as `(CGEventType)(~0)`.
const ANY_INPUT_EVENT: CGEventType = CGEventType(u32::MAX);

pub(super) fn presence() -> Option<Presence> {
    let session = CGSessionCopyCurrentDictionary()?;
    // SAFETY: CGSession's dictionary is keyed by CFString with CF values.
    let props = unsafe { properties(&session) };
    // The key exists only while the screen is locked.
    let locked = flag(props, "CGSSessionScreenIsLocked").unwrap_or(false);
    let idle = CGEventSource::seconds_since_last_event_type(
        CGEventSourceStateID::CombinedSessionState,
        ANY_INPUT_EVENT,
    );
    Some(Presence {
        locked,
        idle: Seconds(idle),
    })
}

pub(super) fn power() -> Option<Power> {
    let battery = smart_battery()?;
    // SAFETY: IORegistry property dictionaries are keyed by CFString with CF
    // values.
    let props = unsafe { properties(&battery) };
    let external = flag(props, "ExternalConnected").unwrap_or(false);
    Some(Power {
        source: if external {
            Source::Ac
        } else {
            Source::Battery
        },
        percent: percent(int(props, "CurrentCapacity")?, int(props, "MaxCapacity")?)?,
        charging: flag(props, "IsCharging").unwrap_or(false),
        current: Milliamps(i32::try_from(int(props, "Amperage")?).ok()?),
        voltage: Millivolts(u32::try_from(int(props, "Voltage")?).ok()?),
        adapter: if external { adapter(props) } else { None },
    })
}

/// Apple silicon reports `CurrentCapacity` as a percentage against a
/// `MaxCapacity` of 100; Intel Macs report both in mAh. The ratio is right
/// for either.
fn percent(current: i64, max: i64) -> Option<Percent> {
    if max <= 0 || current < 0 {
        return None;
    }
    let rounded = (current * 100 + max / 2) / max;
    u8::try_from(rounded.min(100)).ok().map(Percent)
}

fn adapter(props: &Properties) -> Option<Watts> {
    let details = get(props, "AdapterDetails")?;
    let details = details.downcast_ref::<CFDictionary>()?;
    // SAFETY: a nested IORegistry dictionary has the same key and value types
    // as its parent.
    let details = unsafe { properties(details) };
    u32::try_from(int(details, "Watts")?).ok().map(Watts)
}

/// Copy the `AppleSmartBattery` service's properties; `None` without a battery.
fn smart_battery() -> Option<CFRetained<CFMutableDictionary>> {
    // SAFETY: the argument is a NUL-terminated C string literal.
    let matching = unsafe { IOServiceMatching(c"AppleSmartBattery".as_ptr()) }?;
    let matching: CFRetained<CFDictionary> = CFRetained::from(&*matching);
    // SAFETY: 0 is `kIOMainPortDefault`. The call consumes one reference to
    // `matching`, which the binding models by taking it by value.
    let service = unsafe { IOServiceGetMatchingService(0, Some(matching)) };
    if service == 0 {
        return None;
    }
    let mut raw: *mut CFMutableDictionary = ptr::null_mut();
    // SAFETY: `raw` is a valid out-pointer for the duration of the call.
    let status = unsafe { IORegistryEntryCreateCFProperties(service, &raw mut raw, None, 0) };
    IOObjectRelease(service);
    if status != 0 {
        return None;
    }
    let raw = NonNull::new(raw)?;
    // SAFETY: on success the function hands over one reference under the
    // Create rule, and `CFRetained` takes ownership of exactly that one.
    Some(unsafe { CFRetained::from_raw(raw) })
}

/// View a CF dictionary as string-keyed properties.
///
/// # Safety
///
/// Every key in `dict` must be a `CFString` and every value a CF object, as
/// in `CGSession` and `IORegistry` property dictionaries.
unsafe fn properties(dict: &CFDictionary) -> &Properties {
    // SAFETY: the caller guarantees the key and value types.
    unsafe { dict.cast_unchecked() }
}

fn get(props: &Properties, key: &str) -> Option<CFRetained<CFType>> {
    props.get(&CFString::from_str(key))
}

fn int(props: &Properties, key: &str) -> Option<i64> {
    get(props, key)?.downcast_ref::<CFNumber>()?.as_i64()
}

/// Read a boolean stored either as a `CFBoolean` or as a `CFNumber`.
fn flag(props: &Properties, key: &str) -> Option<bool> {
    let value = get(props, key)?;
    if let Some(b) = value.downcast_ref::<CFBoolean>() {
        return Some(b.as_bool());
    }
    Some(value.downcast_ref::<CFNumber>()?.as_i64()? != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_handles_both_capacity_units() {
        assert_eq!(percent(78, 100), Some(Percent(78)));
        assert_eq!(percent(3_510, 4_500), Some(Percent(78)));
        assert_eq!(percent(4_600, 4_500), Some(Percent(100)));
        assert_eq!(percent(0, 4_500), Some(Percent(0)));
    }

    #[test]
    fn percent_rejects_nonsense() {
        assert_eq!(percent(50, 0), None);
        assert_eq!(percent(-1, 100), None);
    }

    #[test]
    fn read_returns_plausible_values() {
        if let Some(power) = power() {
            assert!(power.percent.0 <= 100);
            assert!(power.voltage.0 > 0);
        }
        if let Some(presence) = presence() {
            assert!(presence.idle.0 >= 0.0);
        }
    }
}
