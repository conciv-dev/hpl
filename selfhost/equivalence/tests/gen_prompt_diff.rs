//! Equivalence gate for the `cmd_gen` module's pure prompt-diff slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli` `cmd_gen`
//! module (rust/crates/napl-cli/src/cmd_gen.rs — the `tests` module) against the
//! NAPL-generated `gen_prompt_diff` crate, which composes on the generated
//! `incremental` crate by path.

use gen_prompt_diff::compute_prompt_diff;
use incremental::diff_body_lines;

#[test]
fn compute_prompt_diff_empty_when_no_prior_or_unchanged() {
    assert_eq!(compute_prompt_diff(None, "body"), "");
    assert_eq!(compute_prompt_diff(Some("body"), "body"), "");
}

#[test]
fn compute_prompt_diff_uses_body_line_diff_when_changed() {
    assert_eq!(
        compute_prompt_diff(Some("old line"), "new line"),
        diff_body_lines("old line", "new line").unified
    );
}
