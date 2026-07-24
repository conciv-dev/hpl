//! Generation-time drift detection (the I/O counterpart of `drift.ts`); the
//! report formatting lives in `napl_core::drift`.
//!
//! Stage1: the pure journal-patch replay (`reconstruct_file_content`) is the
//! NAPL-generated `driftdetect_replay` crate, and the on-disk drift walk
//! (`detect_gen_drift`) is the NAPL-generated `driftdetect_io` crate — both
//! re-exported here behind the unchanged public surface. The unit corpus below
//! rides along as the regression net.

use std::collections::BTreeMap;
use std::path::Path;

use napl_core::drift::ModuleDrift;
use napl_core::schemas::{JournalEntry, NaplMap};

use crate::error::{CliError, CliResult};

pub use driftdetect_replay::reconstruct_file_content;

/// Detect drifted, attributed files for a target, mirroring `detectGenDrift`.
pub fn detect_gen_drift(
    root: &Path,
    target: &str,
    map: &NaplMap,
    journal: &[JournalEntry],
    module_scope: Option<&str>,
    prompt_paths: &BTreeMap<String, String>,
) -> CliResult<Vec<ModuleDrift>> {
    driftdetect_io::detect_gen_drift(root, target, map, journal, module_scope, prompt_paths)
        .map_err(CliError::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use napl_core::schemas::{JournalFile, JournalMode};

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
}
