//! Equivalence gate for the `clock` module's pure formatting slice.
//!
//! This is the EXACT unit-test corpus of the hand-written `napl-cli` `clock`
//! module (rust/crates/napl-cli/src/clock.rs — the `iso_from_millis` tests),
//! replayed against the NAPL-generated `clock_fmt` crate under
//! selfhost/.napl/src/rust/clock_fmt/.

use clock_fmt::iso_from_millis;

#[test]
fn epoch_is_unix_zero() {
    assert_eq!(iso_from_millis(0), "1970-01-01T00:00:00.000Z");
}

#[test]
fn known_timestamp_round_trips() {
    // 2026-07-23T00:00:00.000Z == 1784764800 seconds since epoch.
    assert_eq!(iso_from_millis(1_784_764_800_000), "2026-07-23T00:00:00.000Z");
}

#[test]
fn millis_component_is_kept() {
    assert_eq!(iso_from_millis(1_234), "1970-01-01T00:00:01.234Z");
}
