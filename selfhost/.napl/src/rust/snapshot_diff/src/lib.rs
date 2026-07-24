//! Diffing two directory snapshots.
//!
//! Pure comparison core of the CLI's snapshot module: given a before and an
//! after snapshot (each a map from path to content hash), reports which paths
//! changed. No filesystem access; the I/O shell builds the snapshots.

use std::collections::BTreeMap;

/// The sorted list of paths whose value in `after` differs from `before`.
///
/// A path is kept when `before` has no entry for it (newly added) or has a
/// different hash for it (changed). A path present in `before` but absent
/// from `after` is not reported — this diff is after-relative.
pub fn diff_snapshots(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Vec<String> {
    after
        .iter()
        .filter(|(path, hash)| before.get(*path) != Some(*hash))
        .map(|(path, _)| path.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn spec_example() {
        let before = map(&[("/a", "h1"), ("/b", "h2")]);
        let after = map(&[("/a", "h1"), ("/b", "h2x"), ("/c", "h3")]);
        assert_eq!(diff_snapshots(&before, &after), vec!["/b", "/c"]);
    }

    #[test]
    fn empty_before_and_after() {
        let before = BTreeMap::new();
        let after = BTreeMap::new();
        assert!(diff_snapshots(&before, &after).is_empty());
    }

    #[test]
    fn all_new_paths() {
        let before = BTreeMap::new();
        let after = map(&[("/a", "h1"), ("/b", "h2")]);
        assert_eq!(diff_snapshots(&before, &after), vec!["/a", "/b"]);
    }

    #[test]
    fn identical_snapshots_yield_nothing() {
        let before = map(&[("/a", "h1"), ("/b", "h2")]);
        let after = before.clone();
        assert!(diff_snapshots(&before, &after).is_empty());
    }

    #[test]
    fn removed_path_not_reported() {
        let before = map(&[("/a", "h1"), ("/b", "h2")]);
        let after = map(&[("/a", "h1")]);
        assert!(diff_snapshots(&before, &after).is_empty());
    }

    #[test]
    fn result_is_sorted() {
        let before = BTreeMap::new();
        let after = map(&[("/z", "h1"), ("/a", "h2"), ("/m", "h3")]);
        assert_eq!(diff_snapshots(&before, &after), vec!["/a", "/m", "/z"]);
    }

    #[test]
    fn mixed_add_change_unchanged_remove() {
        let before = map(&[("/a", "h1"), ("/b", "h2"), ("/d", "h4")]);
        let after = map(&[("/a", "h1"), ("/b", "h2changed"), ("/c", "h3")]);
        assert_eq!(diff_snapshots(&before, &after), vec!["/b", "/c"]);
    }
}
