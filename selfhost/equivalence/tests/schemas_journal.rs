//! Equivalence gate for the `schemas::journal` module.
//!
//! This is the EXACT unit-test corpus of the hand-written `napl-core`
//! `schemas::journal` module (rust/crates/napl-core/src/schemas/journal.rs),
//! replayed against the NAPL-generated `schemas_journal` crate under
//! selfhost/.napl/src/rust/schemas_journal/. Each case asserts the same input ->
//! output the hand-written module asserts for itself.
//!
//! `file_patch` is the NAPL-generated `text_diff::unified_diff` and `file_history`
//! yields NAPL-generated `blame::BlameSourceEntry` values, so this test also
//! proves the intra-workspace composition `schemas_journal -> {text_diff, blame}`.

use schemas_journal::{
    file_history, file_patch, next_gen_number, read_journal_str, JournalEntry, JournalFile,
    JournalMode,
};

fn entry(gen: i64, path: &str) -> JournalEntry {
    JournalEntry {
        gen,
        timestamp: format!("t{gen}"),
        module: "demo".to_string(),
        target: "react".to_string(),
        prompt_hash: format!("h{gen}"),
        prompt_diff: String::new(),
        mode: JournalMode::Full,
        files: vec![JournalFile {
            path: path.to_string(),
            patch: file_patch(None, "x\n"),
            hash_before: None,
            hash_after: "abc".to_string(),
        }],
    }
}

#[test]
fn file_patch_created_file() {
    let patch = file_patch(None, "a\nb\n");
    assert!(patch.contains("@@ -0,0 +1,2 @@"));
    assert!(patch.contains("+a"));
    assert!(patch.contains("+b"));
}

#[test]
fn file_patch_modified_file() {
    let patch = file_patch(Some("a\nb\nc\n"), "a\nB\nc\n");
    assert!(patch.contains("-b"));
    assert!(patch.contains("+B"));
}

#[test]
fn round_trips_appended_entries() {
    let raw = format!(
        "{}\n{}\n",
        serde_json::to_string(&entry(1, "f.ts")).unwrap(),
        serde_json::to_string(&entry(2, "f.ts")).unwrap()
    );
    let (entries, warnings) = read_journal_str(&raw);
    assert_eq!(entries.iter().map(|e| e.gen).collect::<Vec<_>>(), vec![1, 2]);
    assert!(warnings.is_empty());
}

#[test]
fn empty_journal_is_empty() {
    let (entries, _) = read_journal_str("");
    assert!(entries.is_empty());
}

#[test]
fn skips_corrupt_and_invalid_lines_with_warnings() {
    let valid = serde_json::to_string(&entry(1, "f.ts")).unwrap();
    let invalid_schema = r#"{"gen":"nope","module":"x"}"#;
    let raw = format!("{valid}\nnot json at all\n{invalid_schema}\n");
    let (entries, warnings) = read_journal_str(&raw);
    assert_eq!(entries.iter().map(|e| e.gen).collect::<Vec<_>>(), vec![1]);
    assert_eq!(warnings.len(), 2);
}

#[test]
fn next_gen_number_cases() {
    assert_eq!(next_gen_number(&[]), 1);
    assert_eq!(
        next_gen_number(&[entry(3, "a"), entry(7, "b"), entry(5, "c")]),
        8
    );
}

#[test]
fn file_history_filters_and_carries_patch() {
    let entries = [entry(1, "a.ts"), entry(2, "b.ts"), entry(3, "a.ts")];
    let history = file_history(&entries, "a.ts");
    assert_eq!(history.iter().map(|h| h.gen).collect::<Vec<_>>(), vec![1, 3]);
    assert!(history[0].patch.contains("+x"));
}

#[test]
fn file_history_empty_for_missing_file() {
    assert!(file_history(&[entry(1, "a.ts")], "missing.ts").is_empty());
}
