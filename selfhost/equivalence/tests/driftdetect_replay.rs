//! Equivalence gate for the `driftdetect` module's pure replay slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli`
//! `driftdetect` module (rust/crates/napl-cli/src/driftdetect.rs — the
//! `reconstruct_file_content` tests) against the NAPL-generated
//! `driftdetect_replay` crate. The `JournalEntry` inputs are built from the
//! generated `schemas_journal` crate, so this also proves the composition
//! `driftdetect_replay -> {schemas_journal, text_diff}`.

use driftdetect_replay::reconstruct_file_content;
use schemas_journal::{JournalEntry, JournalFile, JournalMode};

fn entry(gen: i64, patch: &str) -> JournalEntry {
    JournalEntry {
        gen,
        timestamp: format!("t{gen}"),
        module: "greeting".to_string(),
        target: "typescript".to_string(),
        prompt_hash: format!("h{gen}"),
        prompt_diff: String::new(),
        mode: JournalMode::Full,
        files: vec![JournalFile {
            path: ".napl/src/typescript/greet.ts".to_string(),
            patch: patch.to_string(),
            hash_before: None,
            hash_after: "x".to_string(),
        }],
    }
}

#[test]
fn reconstruct_replays_patches_in_order() {
    let entries = vec![
        entry(1, "@@ -0,0 +1,1 @@\n+line one"),
        entry(2, "@@ -1,1 +1,2 @@\n line one\n+line two"),
    ];
    let content = reconstruct_file_content(&entries, ".napl/src/typescript/greet.ts").unwrap();
    assert_eq!(content, "line one\nline two");
}

#[test]
fn reconstruct_returns_none_for_unknown_file() {
    let entries = vec![entry(1, "@@ -0,0 +1,1 @@\n+x")];
    assert!(reconstruct_file_content(&entries, "other.ts").is_none());
}
