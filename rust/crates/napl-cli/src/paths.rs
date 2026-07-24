//! Path resolution and prompt-file discovery (the I/O counterparts of
//! `paths.ts`).
//!
//! Stage1: the pure path algebra (`NaplPaths`, `resolve_paths`, `rel_to`) is the
//! NAPL-generated `paths_core` crate and the discovery filesystem walk
//! (`find_prompt_files`) is the NAPL-generated `paths_walk` crate; both are
//! re-exported here behind the unchanged public surface. The unit corpus below
//! rides along as the regression net.

pub use paths_core::{rel_to, resolve_paths, NaplPaths};
pub use paths_walk::find_prompt_files;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
