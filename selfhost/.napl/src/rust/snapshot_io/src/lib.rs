use hash::content_hash;
use snapshot_filter::SnapshotFilter;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Walk `dir` and record each surviving file's content hash, keyed by its
/// absolute path.
pub fn snapshot_hashes(
    dir: &Path,
    filter: &SnapshotFilter,
) -> Result<BTreeMap<String, String>, String> {
    let mut records = BTreeMap::new();
    walk(dir, filter, true, &mut records, &|content| content_hash(content))?;
    Ok(records)
}

/// Walk `dir` and record each surviving file's raw content, keyed by its
/// absolute path.
pub fn snapshot_contents(
    dir: &Path,
    filter: &SnapshotFilter,
) -> Result<BTreeMap<String, String>, String> {
    let mut records = BTreeMap::new();
    walk(dir, filter, true, &mut records, &|content| content.to_string())?;
    Ok(records)
}

fn walk(
    dir: &Path,
    filter: &SnapshotFilter,
    at_root: bool,
    records: &mut BTreeMap<String, String>,
    record: &dyn Fn(&str) -> String,
) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if file_type.is_dir() {
            if !filter.is_excluded_dir(&name) {
                walk(&path, filter, false, records, record)?;
            }
        } else if file_type.is_file() && !filter.is_excluded_file(&name, at_root) {
            let content = fs::read_to_string(&path).map_err(|err| err.to_string())?;
            records.insert(path.to_string_lossy().to_string(), record(&content));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use snapshot_filter::make_filter;
    use std::path::PathBuf;
    use std::process;

    struct TmpDir {
        path: PathBuf,
    }

    impl TmpDir {
        fn new(tag: &str) -> TmpDir {
            let mut path = std::env::temp_dir();
            path.push(format!("snapshot_io-{}-{}", process::id(), tag));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TmpDir { path }
        }

        fn write(&self, rel: &str, content: &str) -> PathBuf {
            let target = self.path.join(rel);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(&target, content).unwrap();
            target
        }

        fn key(&self, rel: &str) -> String {
            self.path.join(rel).to_string_lossy().to_string()
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn strs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn excluded_dirs_files_and_suffixes_are_omitted() {
        let dir = TmpDir::new("exclusions");
        dir.write("keep.ts", "keep me");
        dir.write("AGENTS.md", "guidance");
        dir.write("types.d.ts", "declare const x: number;");
        dir.write("node_modules/dep.js", "module.exports = {};");

        let filter = make_filter(
            &strs(&["node_modules"]),
            &strs(&["AGENTS.md"]),
            &[],
            &strs(&[".d.ts"]),
        );
        let snapshot = snapshot_hashes(&dir.path, &filter).unwrap();

        assert_eq!(
            snapshot.keys().cloned().collect::<Vec<String>>(),
            vec![dir.key("keep.ts")]
        );
        assert_eq!(snapshot[&dir.key("keep.ts")], content_hash("keep me"));
    }

    #[test]
    fn root_only_exclusions_keep_nested_namesakes() {
        let dir = TmpDir::new("root-only");
        dir.write("Cargo.toml", "[workspace]");
        dir.write("member/Cargo.toml", "[package]");
        dir.write("member/lib.rs", "pub fn f() {}");

        let filter = make_filter(&[], &[], &strs(&["Cargo.toml"]), &[]);
        let snapshot = snapshot_hashes(&dir.path, &filter).unwrap();

        let mut keys = snapshot.keys().cloned().collect::<Vec<String>>();
        keys.sort();
        assert_eq!(
            keys,
            vec![dir.key("member/Cargo.toml"), dir.key("member/lib.rs")]
        );
    }

    #[test]
    fn content_snapshot_records_raw_content() {
        let dir = TmpDir::new("contents");
        dir.write("a.txt", "alpha");
        dir.write("nested/b.txt", "beta");

        let filter = make_filter(&[], &[], &[], &[]);
        let snapshot = snapshot_contents(&dir.path, &filter).unwrap();

        assert_eq!(snapshot[&dir.key("a.txt")], "alpha");
        assert_eq!(snapshot[&dir.key("nested/b.txt")], "beta");
    }

    #[test]
    fn unreadable_root_yields_no_entries() {
        let dir = TmpDir::new("unreadable");
        let filter = make_filter(&[], &[], &[], &[]);

        let missing = dir.path.join("does-not-exist");
        assert!(snapshot_hashes(&missing, &filter).unwrap().is_empty());
        assert!(snapshot_contents(&missing, &filter).unwrap().is_empty());
    }
}
