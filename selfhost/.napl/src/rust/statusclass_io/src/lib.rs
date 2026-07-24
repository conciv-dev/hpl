//! Per-prompt status classification: reading generated files off disk.
//!
//! Given one prompt's frontmatter identity and the recorded module map, this
//! reads the prompt's generated files off disk and decides the prompt's status
//! — clean, drifted, unattributed, or prompt-stale. This is an I/O shell — it
//! reads generated files to compare their hashes — so it declares no
//! given/expect corpus; its behavior is pinned by the conformance suite (the
//! status scenarios) and by its own tests. The status enum and its one-line
//! rendering live in `statusclass_render`; the frontmatter parse stays in the
//! caller, which passes the already-parsed `module` and declared `targets`.

use hash::content_hash;
use schemas_map::NaplMap;
use statusclass_render::{FileStatus, StatusEntry};
use std::fs;
use std::path::Path;

/// Classify one prompt against the map and the generated files on disk.
///
/// `rel_path` is the prompt path relative to the project root and becomes the
/// entry's `file`; `raw` is the prompt's whole text, whose hash is compared
/// against the hash recorded at generation. The `Err` string is the bare I/O
/// error message, which the CLI shell maps into its own error type unchanged.
pub fn classify(
    root: &Path,
    rel_path: &str,
    raw: &str,
    module: &str,
    targets: &[String],
    map: &NaplMap,
) -> Result<StatusEntry, String> {
    let prompt_hash = content_hash(raw);
    let Some(record) = map.prompts.get(module) else {
        return Ok(entry(rel_path, FileStatus::PromptStale, "never generated"));
    };

    for (target, target_record) in record.targets.iter() {
        if target_record.unattributed == Some(true) {
            continue;
        }
        for file_path in &target_record.files {
            let absolute = root.join(file_path);
            if !absolute.exists() {
                return Ok(entry(
                    rel_path,
                    FileStatus::Drift,
                    &format!("{target}: {file_path} is missing"),
                ));
            }
            let content = fs::read_to_string(&absolute).map_err(|err| err.to_string())?;
            let recorded = map.files.get(file_path).map(|file| file.hash.as_str());
            if recorded != Some(content_hash(&content).as_str()) {
                return Ok(entry(
                    rel_path,
                    FileStatus::Drift,
                    &format!("{target}: {file_path} was edited"),
                ));
            }
        }
    }

    for (target, target_record) in record.targets.iter() {
        if target_record.unattributed == Some(true) {
            return Ok(entry(
                rel_path,
                FileStatus::Unattributed,
                &format!("generated files lack prompt attribution — run napl gen {target} --force"),
            ));
        }
    }

    for target in targets {
        let Some(target_record) = record.targets.get(target) else {
            return Ok(entry(
                rel_path,
                FileStatus::PromptStale,
                &format!("{target}: not generated"),
            ));
        };
        if target_record.prompt_hash_at_gen.as_deref() != Some(prompt_hash.as_str()) {
            return Ok(entry(
                rel_path,
                FileStatus::PromptStale,
                "prompt changed since gen",
            ));
        }
    }

    Ok(entry(rel_path, FileStatus::Clean, ""))
}

fn entry(rel_path: &str, status: FileStatus, detail: &str) -> StatusEntry {
    StatusEntry {
        file: rel_path.to_string(),
        status,
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemas_map::{empty_map, FileRecord, PromptRecord, PromptTargetRecord};
    use std::path::PathBuf;
    use std::process;

    struct TmpDir {
        path: PathBuf,
    }

    impl TmpDir {
        fn new(tag: &str) -> TmpDir {
            let mut path = std::env::temp_dir();
            path.push(format!("statusclass_io-{}-{}", process::id(), tag));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TmpDir { path }
        }

        fn write(&self, rel: &str, content: &str) {
            let target = self.path.join(rel);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(&target, content).unwrap();
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    const RAW: &str = "# greeting\n\nSay hello.\n";
    const GENERATED: &str = "export const greet = () => 'hi';\n";
    const FILE_PATH: &str = "src/ts/greeting.ts";

    fn target_record(
        files: &[&str],
        unattributed: Option<bool>,
        hash_at_gen: &str,
    ) -> PromptTargetRecord {
        PromptTargetRecord {
            prompt_hash_at_gen: Some(hash_at_gen.to_string()),
            files: files.iter().map(|f| f.to_string()).collect(),
            unattributed,
        }
    }

    fn map_with(target: PromptTargetRecord, file_hash: Option<&str>) -> NaplMap {
        let mut map = empty_map();
        let mut record = PromptRecord {
            module: "greeting".to_string(),
            prompt_hash: content_hash(RAW),
            declared_targets: vec!["ts".to_string()],
            targets: Default::default(),
        };
        record.targets.insert("ts".to_string(), target);
        map.prompts.insert("greeting".to_string(), record);
        if let Some(hash) = file_hash {
            map.files.insert(
                FILE_PATH.to_string(),
                FileRecord {
                    target: "ts".to_string(),
                    hash: hash.to_string(),
                    prompts: vec!["greeting".to_string()],
                },
            );
        }
        map
    }

    fn classify_in(dir: &TmpDir, map: &NaplMap) -> StatusEntry {
        classify(
            &dir.path,
            "examples/greeting.napl",
            RAW,
            "greeting",
            &["ts".to_string()],
            map,
        )
        .unwrap()
    }

    #[test]
    fn unknown_module_is_never_generated() {
        let entry = classify(
            Path::new("/nonexistent-root"),
            "examples/greeting.napl",
            RAW,
            "greeting",
            &["ts".to_string()],
            &empty_map(),
        )
        .unwrap();
        assert_eq!(entry.file, "examples/greeting.napl");
        assert_eq!(entry.status, FileStatus::PromptStale);
        assert_eq!(entry.detail, "never generated");
    }

    #[test]
    fn matching_hashes_are_clean() {
        let dir = TmpDir::new("clean");
        dir.write(FILE_PATH, GENERATED);
        let map = map_with(
            target_record(&[FILE_PATH], None, &content_hash(RAW)),
            Some(&content_hash(GENERATED)),
        );
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::Clean);
        assert_eq!(entry.detail, "");
    }

    #[test]
    fn missing_generated_file_is_drift() {
        let dir = TmpDir::new("missing");
        let map = map_with(
            target_record(&[FILE_PATH], None, &content_hash(RAW)),
            Some(&content_hash(GENERATED)),
        );
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::Drift);
        assert_eq!(entry.detail, "ts: src/ts/greeting.ts is missing");
    }

    #[test]
    fn edited_generated_file_is_drift() {
        let dir = TmpDir::new("edited");
        dir.write(FILE_PATH, "export const greet = () => 'edited';\n");
        let map = map_with(
            target_record(&[FILE_PATH], None, &content_hash(RAW)),
            Some(&content_hash(GENERATED)),
        );
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::Drift);
        assert_eq!(entry.detail, "ts: src/ts/greeting.ts was edited");
    }

    #[test]
    fn unrecorded_generated_file_is_drift() {
        let dir = TmpDir::new("unrecorded");
        dir.write(FILE_PATH, GENERATED);
        let map = map_with(target_record(&[FILE_PATH], None, &content_hash(RAW)), None);
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::Drift);
        assert_eq!(entry.detail, "ts: src/ts/greeting.ts was edited");
    }

    #[test]
    fn unattributed_target_skips_the_drift_check_and_reports_attribution() {
        let dir = TmpDir::new("unattributed");
        let map = map_with(
            target_record(&[FILE_PATH], Some(true), &content_hash(RAW)),
            None,
        );
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::Unattributed);
        assert_eq!(
            entry.detail,
            "generated files lack prompt attribution — run napl gen ts --force"
        );
    }

    #[test]
    fn changed_prompt_is_stale() {
        let dir = TmpDir::new("stale");
        dir.write(FILE_PATH, GENERATED);
        let map = map_with(
            target_record(&[FILE_PATH], None, &content_hash("# greeting\n\nSay hi.\n")),
            Some(&content_hash(GENERATED)),
        );
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::PromptStale);
        assert_eq!(entry.detail, "prompt changed since gen");
    }

    #[test]
    fn declared_target_absent_from_the_record_is_stale() {
        let dir = TmpDir::new("not-generated");
        dir.write(FILE_PATH, GENERATED);
        let map = map_with(
            target_record(&[FILE_PATH], None, &content_hash(RAW)),
            Some(&content_hash(GENERATED)),
        );
        let entry = classify(
            &dir.path,
            "examples/greeting.napl",
            RAW,
            "greeting",
            &["ts".to_string(), "rust".to_string()],
            &map,
        )
        .unwrap();
        assert_eq!(entry.status, FileStatus::PromptStale);
        assert_eq!(entry.detail, "rust: not generated");
    }

    #[test]
    fn drift_wins_over_a_changed_prompt() {
        let dir = TmpDir::new("drift-precedence");
        let map = map_with(
            target_record(&[FILE_PATH], None, "some-older-prompt-hash"),
            Some(&content_hash(GENERATED)),
        );
        let entry = classify_in(&dir, &map);
        assert_eq!(entry.status, FileStatus::Drift);
    }

    #[test]
    fn io_error_is_propagated_as_its_display_string() {
        let dir = TmpDir::new("io-error");
        fs::create_dir_all(dir.path.join(FILE_PATH)).unwrap();
        let map = map_with(
            target_record(&[FILE_PATH], None, &content_hash(RAW)),
            Some(&content_hash(GENERATED)),
        );
        let err = classify(
            &dir.path,
            "examples/greeting.napl",
            RAW,
            "greeting",
            &["ts".to_string()],
            &map,
        )
        .unwrap_err();
        assert!(!err.is_empty());
    }
}
