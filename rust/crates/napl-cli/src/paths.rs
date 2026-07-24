//! Path resolution and prompt-file discovery (the I/O counterparts of
//! `paths.ts`).
//!
//! Stage1: the pure path algebra (`NaplPaths`, `resolve_paths`, `rel_to`) is the
//! NAPL-generated `paths_core` crate, re-exported here; this shell keeps only
//! the filesystem walk that discovers prompt files. The unit corpus below rides
//! along as the regression net.

use std::path::{Path, PathBuf};

use napl_core::extensions::is_prompt_file;

pub use paths_core::{rel_to, resolve_paths, NaplPaths};

const IGNORED_DIRS: [&str; 3] = ["node_modules", ".napl", ".git"];

fn walk(dir: &Path, aliases: &[String], results: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let full = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if IGNORED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            walk(&full, aliases, results)?;
        } else if file_type.is_file() && is_prompt_file(&name, Some(aliases)) {
            results.push(full);
        }
    }
    Ok(())
}

/// Discover every prompt file under `root`, sorted, mirroring `findPromptFiles`.
pub fn find_prompt_files(root: &Path, aliases: &[String]) -> std::io::Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    walk(root, aliases, &mut results)?;
    results.sort();
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_to_produces_posix_paths() {
        let root = Path::new("/project");
        assert_eq!(
            rel_to(root, Path::new("/project/.napl/map.json")),
            ".napl/map.json"
        );
    }

    #[test]
    fn find_prompt_files_ignores_state_dirs_and_sorts() {
        let dir = std::env::temp_dir().join(format!("napl-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("examples")).unwrap();
        std::fs::create_dir_all(dir.join(".napl/src")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join("examples/b.napl"), "x").unwrap();
        std::fs::write(dir.join("examples/a.napl"), "x").unwrap();
        std::fs::write(dir.join(".napl/src/ignored.napl"), "x").unwrap();
        std::fs::write(dir.join("node_modules/dep.napl"), "x").unwrap();
        std::fs::write(dir.join("examples/notprompt.txt"), "x").unwrap();
        let aliases = napl_core::extensions::default_prompt_aliases();
        let found = find_prompt_files(&dir, &aliases).unwrap();
        let rels: Vec<String> = found.iter().map(|p| rel_to(&dir, p)).collect();
        assert_eq!(
            rels,
            vec!["examples/a.napl".to_string(), "examples/b.napl".to_string()]
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
