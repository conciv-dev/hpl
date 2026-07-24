//! Equivalence gate for the `cmd_gen` module's pure classification slice.
//!
//! Replays the EXACT unit-test corpus of the hand-written `napl-cli` `cmd_gen`
//! module (rust/crates/napl-cli/src/cmd_gen.rs — the `tests` module) against the
//! NAPL-generated `gen_classify` crate.

use gen_classify::{first_meaningful_line, is_source_file, split_body_lines};

#[test]
fn is_source_file_accepts_known_extensions_and_rejects_config_and_others() {
    assert!(is_source_file("src/lib.rs"));
    assert!(is_source_file("app.tsx"));
    assert!(is_source_file("styles.css"));
    assert!(is_source_file("dir/x.jsx"));
    assert!(is_source_file("page.html"));
    assert!(!is_source_file("vite.config.ts"));
    assert!(!is_source_file("tailwind.config.js"));
    assert!(!is_source_file("README.md"));
    assert!(!is_source_file("noext"));
}

#[test]
fn first_meaningful_line_strips_headings_and_caps_length() {
    assert_eq!(first_meaningful_line("# Title\n\nBody line"), "Title");
    assert_eq!(first_meaningful_line("\n\n  hello world  \n"), "hello world");
    assert_eq!(first_meaningful_line("### Deep\nmore"), "Deep");
    assert_eq!(first_meaningful_line(""), "(no description)");
    assert_eq!(first_meaningful_line("   \n\t\n"), "(no description)");
    let long = "x".repeat(200);
    assert_eq!(first_meaningful_line(&long).chars().count(), 120);
}

#[test]
fn split_body_lines_splits_and_strips_cr() {
    assert_eq!(split_body_lines("a\r\nb\nc"), vec!["a", "b", "c"]);
    assert_eq!(split_body_lines(""), vec![""]);
    assert_eq!(split_body_lines("x\r\n"), vec!["x", ""]);
}
