//! Prompt-file discovery: the filesystem walk.
//!
//! The CLI's prompt-discovery seam: it walks a project directory tree and
//! collects every prompt file, skipping the state and dependency directories
//! that must never be treated as source. This is an I/O shell — it reads the
//! real filesystem — so it declares no given/expect corpus; its behavior is
//! pinned by the conformance suite and by its own filesystem walk test. The
//! pure path algebra (`resolve_paths`, `NaplPaths`, `rel_to`) lives in a
//! separate crate and is not part of this module.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Directory names never descended into during discovery: dependency and
/// toolchain-state trees, never source.
const IGNORED_DIRS: [&str; 3] = ["node_modules", ".napl", ".git"];

/// Discover every prompt file under `root`, sorted by the default path ordering.
///
/// Walks the tree rooted at `root`, collecting each entry that is a file whose
/// name is a prompt file per [`extensions::is_prompt_file`] with the given
/// `aliases`. Directories named `node_modules`, `.napl`, or `.git` are skipped
/// without descending.
///
/// A directory that does not exist is treated as empty rather than an error, so
/// discovering under a missing `root` yields an empty list. Any other I/O error
/// is propagated.
pub fn find_prompt_files(root: &Path, aliases: &[String]) -> io::Result<Vec<PathBuf>> {
    let borrowed: Vec<&str> = aliases.iter().map(String::as_str).collect();
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(dir) = pending.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        };
        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if file_type.is_dir() {
                if !IGNORED_DIRS.contains(&name.as_ref()) {
                    pending.push(entry.path());
                }
            } else if file_type.is_file() && extensions::is_prompt_file(&name, Some(&borrowed)) {
                found.push(entry.path());
            }
        }
    }

    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before the unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "paths_walk-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn finds_prompts_and_skips_ignored_trees() {
        let root = unique_temp_dir("walk");

        fs::create_dir_all(root.join("examples")).expect("create examples");
        fs::write(root.join("examples/a.napl"), "a").expect("write a.napl");
        fs::write(root.join("examples/b.napl"), "b").expect("write b.napl");
        fs::write(root.join("examples/notprompt.txt"), "x").expect("write notprompt.txt");

        fs::create_dir_all(root.join(".napl/src")).expect("create .napl/src");
        fs::write(root.join(".napl/src/ignored.napl"), "i").expect("write ignored.napl");

        fs::create_dir_all(root.join("node_modules")).expect("create node_modules");
        fs::write(root.join("node_modules/dep.napl"), "d").expect("write dep.napl");

        let found = find_prompt_files(&root, &extensions::default_prompt_aliases())
            .expect("discovery succeeds");

        assert_eq!(
            found,
            vec![root.join("examples/a.napl"), root.join("examples/b.napl")]
        );

        fs::remove_dir_all(&root).expect("clean up temp dir");
    }

    #[test]
    fn missing_root_yields_an_empty_list() {
        let root = unique_temp_dir("missing");
        let absent = root.join("nope");

        let found = find_prompt_files(&absent, &extensions::default_prompt_aliases())
            .expect("a missing directory is empty, not an error");
        assert!(found.is_empty());

        fs::remove_dir_all(&root).expect("clean up temp dir");
    }
}
