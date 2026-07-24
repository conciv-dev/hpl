//! Equivalence gate for the `cmd_gen` module's pure attribution-sanity slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli` `cmd_gen`
//! module (rust/crates/napl-cli/src/cmd_gen.rs — the `tests` module) against the
//! NAPL-generated `gen_attribution_check` crate, which composes on the generated
//! `schemas_attribution` (and its `schemas_line_range`) crates by path.

use gen_attribution_check::assert_attribution_sane;
use schemas_attribution::{Attribution, AttributionEntry};
use schemas_line_range::LineRange;

fn entry(file: &str) -> AttributionEntry {
    AttributionEntry {
        prompt_lines: LineRange::new(1, 1),
        file: file.to_string(),
        lines: LineRange::new(1, 1),
        note: String::new(),
    }
}

fn attribution(entries: Vec<AttributionEntry>) -> Attribution {
    Attribution {
        module: "m".to_string(),
        target: "rust".to_string(),
        entries,
    }
}

#[test]
fn assert_attribution_sane_ok_when_no_files_and_no_entries() {
    assert_eq!(assert_attribution_sane(&attribution(vec![]), &[]), Ok(()));
}

#[test]
fn assert_attribution_sane_rejects_empty_entries_with_attributed_files() {
    assert_eq!(
        assert_attribution_sane(&attribution(vec![]), &["a.ts".to_string()]),
        Err("attribution has no entries but the module has attributed source files".to_string())
    );
}

#[test]
fn assert_attribution_sane_ok_when_entry_in_allowed_set() {
    assert_eq!(
        assert_attribution_sane(&attribution(vec![entry("a.ts")]), &["a.ts".to_string()]),
        Ok(())
    );
}

#[test]
fn assert_attribution_sane_rejects_entry_outside_allowed_set() {
    assert_eq!(
        assert_attribution_sane(&attribution(vec![entry("b.ts")]), &["a.ts".to_string()]),
        Err("attribution entry references file \"b.ts\" which is outside the attributed file set".to_string())
    );
}
