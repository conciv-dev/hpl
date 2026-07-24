//! Equivalence gate for the `incremental` module.
//!
//! This replays the hand-written `napl-core` `incremental` corpus
//! (rust/crates/napl-core/src/incremental.rs) against the NAPL-generated
//! `incremental` crate under selfhost/.napl/src/rust/incremental/. The two
//! hand-written unit tests are replayed verbatim; one extra case exercises
//! `select_intersecting_entries` over `AttributionEntry` values constructed from
//! the NAPL-generated sibling crates, proving the type-real composition
//! `incremental -> {schemas_attribution, schemas_line_range}`.

use incremental::{diff_body_lines, incremental_unlock_list, select_intersecting_entries};
use schemas_attribution::AttributionEntry;
use schemas_line_range::LineRange;

#[test]
fn diff_tracks_changed_lines_and_unified() {
    let diff = diff_body_lines(
        "Greet a person by name.\n",
        "Greet a person by name, loudly.\n",
    );
    assert!(diff.unified.contains("-Greet a person by name."));
    assert!(diff.unified.contains("+Greet a person by name, loudly."));
    assert_eq!(diff.changed_old_lines, vec![1]);
    assert_eq!(diff.changed_new_lines, vec![1]);
}

#[test]
fn unlock_list_is_sorted_and_deduped() {
    let list = incremental_unlock_list(
        &[".napl/src/typescript/greet.ts".to_string()],
        &[],
        ".napl/src/typescript",
    );
    assert_eq!(list, vec![".napl/src/typescript/greet.ts".to_string()]);
}

#[test]
fn select_intersecting_matches_changed_prompt_lines() {
    let entries = vec![
        AttributionEntry {
            prompt_lines: LineRange::new(1, 2),
            file: "a.ts".to_string(),
            lines: LineRange::new(1, 1),
            note: "covers line one".to_string(),
        },
        AttributionEntry {
            prompt_lines: LineRange::new(5, 7),
            file: "b.ts".to_string(),
            lines: LineRange::new(3, 4),
            note: "covers later lines".to_string(),
        },
    ];
    let selected = select_intersecting_entries(&entries, &[2]);
    assert_eq!(
        selected.iter().map(|e| e.file.clone()).collect::<Vec<_>>(),
        vec!["a.ts".to_string()]
    );
    assert!(select_intersecting_entries(&entries, &[9]).is_empty());
    let unlocked = incremental_unlock_list(
        &[".napl/src/typescript/keep.ts".to_string()],
        &selected,
        ".napl/src/typescript",
    );
    assert_eq!(
        unlocked,
        vec![
            ".napl/src/typescript/a.ts".to_string(),
            ".napl/src/typescript/keep.ts".to_string(),
        ]
    );
}
