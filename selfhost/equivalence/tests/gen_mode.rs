//! Equivalence gate for the `cmd_gen` module's pure mode-selection slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli` `cmd_gen`
//! module (rust/crates/napl-cli/src/cmd_gen.rs — the `tests` module) against the
//! NAPL-generated `gen_mode` crate. The generated `full_mode_message` takes its
//! `FullModeReason` by value where the hand-written helper took it by reference;
//! the equivalence is behavioral (identical strings), the by-value/by-ref seam is
//! bridged at the shell's call sites.

use gen_mode::{can_incremental, full_mode_message, incremental_mode_message, FullModeReason};

#[test]
fn can_incremental_requires_record_hash_not_full_not_unattributed() {
    assert!(can_incremental(false, true, false, true));
    assert!(!can_incremental(true, true, false, true));
    assert!(!can_incremental(false, false, false, true));
    assert!(!can_incremental(false, true, true, true));
    assert!(!can_incremental(false, true, false, false));
}

#[test]
fn mode_messages_render_exact_strings() {
    assert_eq!(
        incremental_mode_message(3, 2),
        "  mode: INCREMENTAL — 3 changed prompt line(s), 2 owned region(s) affected"
    );
    assert_eq!(
        full_mode_message(FullModeReason::NoPriorOnDisk),
        "  mode: full (no prior prompt body or attribution on disk to diff against)"
    );
    assert_eq!(
        full_mode_message(FullModeReason::ForcedFull),
        "  mode: full (forced --full)"
    );
    assert_eq!(
        full_mode_message(FullModeReason::NoPriorGen),
        "  mode: full (no prior successful gen for this target)"
    );
}
