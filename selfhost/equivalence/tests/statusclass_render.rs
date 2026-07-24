//! Equivalence gate for the `statusclass` module's pure render slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli`
//! `statusclass` module (rust/crates/napl-cli/src/statusclass.rs — the pure
//! `StatusEntry::line` and `is_error` tests) against the NAPL-generated
//! `statusclass_render` crate.

use statusclass_render::{FileStatus, StatusEntry};

fn entry(status: FileStatus, detail: &str) -> StatusEntry {
    StatusEntry {
        file: "examples/greeting.napl".to_string(),
        status,
        detail: detail.to_string(),
    }
}

#[test]
fn line_pads_status_to_twelve() {
    assert_eq!(
        entry(FileStatus::Clean, "").line(),
        "clean        examples/greeting.napl"
    );
    assert_eq!(
        entry(FileStatus::PromptStale, "never generated").line(),
        "prompt-stale examples/greeting.napl (never generated)"
    );
    assert_eq!(
        entry(FileStatus::Drift, "typescript: x was edited").line(),
        "DRIFT        examples/greeting.napl (typescript: x was edited)"
    );
    assert_eq!(
        entry(FileStatus::Unattributed, "run napl gen typescript --force").line(),
        "unattributed examples/greeting.napl (run napl gen typescript --force)"
    );
}

#[test]
fn error_statuses_flagged() {
    assert!(entry(FileStatus::Drift, "").is_error());
    assert!(entry(FileStatus::Unattributed, "").is_error());
    assert!(!entry(FileStatus::Clean, "").is_error());
    assert!(!entry(FileStatus::PromptStale, "").is_error());
}
