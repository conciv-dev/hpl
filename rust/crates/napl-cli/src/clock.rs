//! The clock seam: `now()` reads `NAPL_FIXED_NOW` at the CLI entry, falling
//! back to the real UTC time in ISO-8601 with milliseconds.
//!
//! Stage1: the pure millis-to-ISO formatting is the NAPL-generated `clock_fmt`
//! crate; this shell only reads the wall clock (or the fixed-now env) and
//! delegates the formatting. The unit corpus below rides along as the
//! regression net.

use std::time::{SystemTime, UNIX_EPOCH};

use clock_fmt::iso_from_millis;

/// The current timestamp: `NAPL_FIXED_NOW` when set, else real UTC.
#[must_use]
pub fn now() -> String {
    if let Ok(fixed) = std::env::var("NAPL_FIXED_NOW") {
        return fixed;
    }
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    iso_from_millis(duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_unix_zero() {
        assert_eq!(iso_from_millis(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn known_timestamp_round_trips() {
        // 2026-07-23T00:00:00.000Z == 1784764800 seconds since epoch.
        assert_eq!(
            iso_from_millis(1_784_764_800_000),
            "2026-07-23T00:00:00.000Z"
        );
    }

    #[test]
    fn millis_component_is_kept() {
        assert_eq!(iso_from_millis(1_234), "1970-01-01T00:00:01.234Z");
    }
}
