//! Equivalence gate for the `snapshot` module's pure diff slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli` `snapshot`
//! module (rust/crates/napl-cli/src/snapshot.rs — the `diff_snapshots` test)
//! against the NAPL-generated `snapshot_diff` crate.

use std::collections::BTreeMap;

use snapshot_diff::diff_snapshots;

#[test]
fn diff_reports_added_and_changed() {
    let mut before = BTreeMap::new();
    before.insert("/a".to_string(), "h1".to_string());
    before.insert("/b".to_string(), "h2".to_string());
    let mut after = BTreeMap::new();
    after.insert("/a".to_string(), "h1".to_string());
    after.insert("/b".to_string(), "h2x".to_string());
    after.insert("/c".to_string(), "h3".to_string());
    assert_eq!(
        diff_snapshots(&before, &after),
        vec!["/b".to_string(), "/c".to_string()]
    );
}
