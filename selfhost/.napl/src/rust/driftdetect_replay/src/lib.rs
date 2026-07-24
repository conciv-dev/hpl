//! Reconstructing a generated file's baseline content from journal patches.
//!
//! Pure baseline-reconstruction core of the generation-time drift detector:
//! given journal entries and a file path, replay that file's recorded patches
//! in gen order to rebuild the content the toolchain last wrote. No filesystem
//! access — the I/O shell reads the current file and diffs it against this
//! baseline.

use schemas_journal::JournalEntry;

/// Replay the patches recorded for `file_path` across `entries`, oldest gen
/// first, returning the reconstructed content or `None` if no entry ever
/// touched `file_path`.
pub fn reconstruct_file_content(entries: &[JournalEntry], file_path: &str) -> Option<String> {
    let mut ordered: Vec<&JournalEntry> = entries.iter().collect();
    ordered.sort_by_key(|entry| entry.gen);

    let mut content: Option<String> = None;
    for entry in ordered {
        if let Some(file) = entry.files.iter().find(|f| f.path == file_path) {
            let base = content.unwrap_or_default();
            let hunks = text_diff::parse_hunks(&file.patch);
            content = Some(text_diff::apply_hunks(&base, &hunks));
        }
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemas_journal::{JournalFile, JournalMode};

    fn entry(gen: i64, path: &str, before: &str, after: &str) -> JournalEntry {
        JournalEntry {
            gen,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            module: "m".to_string(),
            target: "t".to_string(),
            prompt_hash: "h".to_string(),
            prompt_diff: "".to_string(),
            mode: JournalMode::Full,
            files: vec![JournalFile {
                path: path.to_string(),
                patch: text_diff::unified_diff(before, after),
                hash_before: None,
                hash_after: "h".to_string(),
            }],
        }
    }

    #[test]
    fn replays_two_patches_in_gen_order() {
        let entries = vec![
            entry(1, "a.rs", "", "line one"),
            entry(2, "a.rs", "line one", "line one\nline two"),
        ];
        assert_eq!(
            reconstruct_file_content(&entries, "a.rs"),
            Some("line one\nline two".to_string())
        );
    }

    #[test]
    fn sorts_out_of_order_entries_without_mutating_input() {
        let entries = vec![
            entry(2, "a.rs", "line one", "line one\nline two"),
            entry(1, "a.rs", "", "line one"),
        ];
        let original = entries.clone();
        assert_eq!(
            reconstruct_file_content(&entries, "a.rs"),
            Some("line one\nline two".to_string())
        );
        assert_eq!(entries, original);
    }

    #[test]
    fn returns_none_for_untouched_path() {
        let entries = vec![entry(1, "a.rs", "", "line one")];
        assert_eq!(reconstruct_file_content(&entries, "b.rs"), None);
    }

    #[test]
    fn ignores_files_from_other_entries_with_different_paths() {
        let mut e1 = entry(1, "a.rs", "", "content a");
        e1.files.push(JournalFile {
            path: "b.rs".to_string(),
            patch: text_diff::unified_diff("", "content b"),
            hash_before: None,
            hash_after: "h".to_string(),
        });
        let entries = vec![e1];
        assert_eq!(
            reconstruct_file_content(&entries, "a.rs"),
            Some("content a".to_string())
        );
        assert_eq!(
            reconstruct_file_content(&entries, "b.rs"),
            Some("content b".to_string())
        );
    }

    #[test]
    fn three_entries_apply_in_ascending_gen_order() {
        let entries = vec![
            entry(3, "a.rs", "v1\nv2", "v1\nv2\nv3"),
            entry(1, "a.rs", "", "v1"),
            entry(2, "a.rs", "v1", "v1\nv2"),
        ];
        assert_eq!(
            reconstruct_file_content(&entries, "a.rs"),
            Some("v1\nv2\nv3".to_string())
        );
    }
}
