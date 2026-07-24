//! Per-prompt status classification (the I/O counterpart of `status-core.ts`).
//!
//! Stage1: the pure status enum and one-line rendering (`FileStatus`,
//! `StatusEntry`, `line`, `is_error`) are the NAPL-generated `statusclass_render`
//! crate and the fs classification (`classify`, reading generated files off disk)
//! is the NAPL-generated `statusclass_io` crate — both re-exported/wrapped here
//! behind the unchanged public surface. The frontmatter parse (and its bridged
//! error text) stays in this shell. The unit corpus below rides along as the
//! regression net.

use std::path::Path;

use napl_core::schemas::{parse_frontmatter, NaplMap};

use crate::error::{CliError, CliResult};

pub use statusclass_render::StatusEntry;

/// Classify one prompt, mirroring `classifyPrompt`.
pub fn classify_prompt(
    root: &Path,
    rel_path: &str,
    raw: &str,
    map: &NaplMap,
) -> CliResult<StatusEntry> {
    let parsed = parse_frontmatter(raw)?;
    statusclass_io::classify(
        root,
        rel_path,
        raw,
        &parsed.frontmatter.module,
        &parsed.frontmatter.targets,
        map,
    )
    .map_err(CliError::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use statusclass_render::FileStatus;

    fn entry(status: FileStatus, detail: &str) -> StatusEntry {
        StatusEntry {
            file: "examples/greeting.napl".to_string(),
            status,
            detail: detail.to_string(),
        }
    }

    #[test]
    fn line_pads_status_to_twelve() {
        assert_eq!(
            entry(FileStatus::Clean, "").line(),
            "clean        examples/greeting.napl"
        );
        assert_eq!(
            entry(FileStatus::PromptStale, "never generated").line(),
            "prompt-stale examples/greeting.napl (never generated)"
        );
        assert_eq!(
            entry(FileStatus::Drift, "typescript: x was edited").line(),
            "DRIFT        examples/greeting.napl (typescript: x was edited)"
        );
        assert_eq!(
            entry(FileStatus::Unattributed, "run napl gen typescript --force").line(),
            "unattributed examples/greeting.napl (run napl gen typescript --force)"
        );
    }

    #[test]
    fn error_statuses_flagged() {
        assert!(entry(FileStatus::Drift, "").is_error());
        assert!(entry(FileStatus::Unattributed, "").is_error());
        assert!(!entry(FileStatus::Clean, "").is_error());
        assert!(!entry(FileStatus::PromptStale, "").is_error());
    }
}
