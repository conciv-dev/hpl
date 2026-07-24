//! Directory snapshots (hashes and contents) and their diff, mirroring
//! `snapshot.ts`.
//!
//! Stage1: the pure snapshot comparison (`diff_snapshots`) is the NAPL-generated
//! `snapshot_diff` crate, the pure exclusion filter (`SnapshotFilter`,
//! `make_filter`) is the NAPL-generated `snapshot_filter` crate, and the
//! filesystem walk (`snapshot_hashes`/`snapshot_contents`) is the NAPL-generated
//! `snapshot_io` crate — all re-exported here behind the unchanged public
//! surface. The unit corpus below rides along as the regression net.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{CliError, CliResult};

pub use snapshot_diff::diff_snapshots;
pub use snapshot_filter::{make_filter, SnapshotFilter};

/// Snapshot the content hashes of a tree, mirroring `snapshotHashes`.
pub fn snapshot_hashes(dir: &Path, filter: &SnapshotFilter) -> CliResult<BTreeMap<String, String>> {
    snapshot_io::snapshot_hashes(dir, filter).map_err(CliError::new)
}

/// Snapshot the contents of a tree, mirroring `snapshotContents`.
pub fn snapshot_contents(
    dir: &Path,
    filter: &SnapshotFilter,
) -> CliResult<BTreeMap<String, String>> {
    snapshot_io::snapshot_contents(dir, filter).map_err(CliError::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_reports_added_and_changed() {
        let mut before = BTreeMap::new();
        before.insert("/a".to_string(), "h1".to_string());
        before.insert("/b".to_string(), "h2".to_string());
        let mut after = BTreeMap::new();
        after.insert("/a".to_string(), "h1".to_string());
        after.insert("/b".to_string(), "h2x".to_string());
        after.insert("/c".to_string(), "h3".to_string());
        assert_eq!(
            diff_snapshots(&before, &after),
            vec!["/b".to_string(), "/c".to_string()]
        );
    }

    #[test]
    fn filter_predicate_decides_dirs_files_roots_and_suffixes() {
        let filter = make_filter(
            &["node_modules".to_string(), ".git".to_string()],
            &["AGENTS.md".to_string()],
            &["Cargo.toml".to_string()],
            &[".d.ts".to_string(), ".lock".to_string()],
        );
        assert!(filter.is_excluded_dir("node_modules"));
        assert!(filter.is_excluded_dir(".git"));
        assert!(!filter.is_excluded_dir("src"));
        assert!(filter.is_excluded_file("AGENTS.md", false));
        assert!(filter.is_excluded_file("AGENTS.md", true));
        assert!(filter.is_excluded_file("types.d.ts", false));
        assert!(filter.is_excluded_file("Cargo.lock", true));
        assert!(!filter.is_excluded_file("keep.ts", false));
        assert!(filter.is_excluded_file("Cargo.toml", true));
        assert!(!filter.is_excluded_file("Cargo.toml", false));
    }

    #[test]
    fn filter_excludes_dirs_files_and_suffixes() {
        let dir = std::env::temp_dir().join(format!("napl-snap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join("keep.ts"), "x").unwrap();
        std::fs::write(dir.join("AGENTS.md"), "y").unwrap();
        std::fs::write(dir.join("types.d.ts"), "z").unwrap();
        std::fs::write(dir.join("node_modules/dep.js"), "n").unwrap();
        let filter = make_filter(
            &["node_modules".to_string()],
            &["AGENTS.md".to_string()],
            &[],
            &[".d.ts".to_string()],
        );
        let hashes = snapshot_hashes(&dir, &filter).unwrap();
        let names: Vec<String> = hashes
            .keys()
            .map(|k| k.rsplit('/').next().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["keep.ts".to_string()]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn root_only_exclusion_keeps_nested_namesakes() {
        let dir = std::env::temp_dir().join(format!("napl-snap-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("member")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "root").unwrap();
        std::fs::write(dir.join("member/Cargo.toml"), "member").unwrap();
        std::fs::write(dir.join("member/lib.rs"), "code").unwrap();
        let filter = make_filter(&[], &[], &["Cargo.toml".to_string()], &[]);
        let hashes = snapshot_hashes(&dir, &filter).unwrap();
        let root_manifest = dir.join("Cargo.toml").to_string_lossy().into_owned();
        let member_manifest = dir.join("member/Cargo.toml").to_string_lossy().into_owned();
        let member_lib = dir.join("member/lib.rs").to_string_lossy().into_owned();
        assert!(!hashes.contains_key(&root_manifest));
        assert!(hashes.contains_key(&member_manifest));
        assert!(hashes.contains_key(&member_lib));
        std::fs::remove_dir_all(&dir).ok();
    }
}
