//! Equivalence gate for the `paths` module's pure path-algebra slice.
//!
//! Replays the hand-written `napl-cli` `paths` unit corpus
//! (rust/crates/napl-cli/src/paths.rs — the `rel_to` test) against the
//! NAPL-generated `paths_core` crate, plus the `resolve_paths` layout the
//! hand-written module builds by hand.

use std::path::Path;

use paths_core::{rel_to, resolve_paths};

#[test]
fn rel_to_produces_posix_paths() {
    let root = Path::new("/project");
    assert_eq!(
        rel_to(root, Path::new("/project/.napl/map.json")),
        ".napl/map.json"
    );
}

#[test]
fn resolve_paths_matches_the_napl_layout() {
    let paths = resolve_paths(Path::new("/p"));
    assert_eq!(paths.ir_dir, Path::new("/p/.napl/ir"));
    assert_eq!(paths.src_dir, Path::new("/p/.napl/src"));
    assert_eq!(paths.map_path, Path::new("/p/.napl/map.json"));
    assert_eq!(paths.lock_path, Path::new("/p/.napl/lock.json"));
    assert_eq!(paths.gen_lock_path, Path::new("/p/.napl/gen.lock"));
    assert_eq!(paths.journal_path, Path::new("/p/.napl/journal.jsonl"));
    assert_eq!(paths.prompts_at_gen_dir, Path::new("/p/.napl/prompts-at-gen"));
    assert_eq!(paths.examples_dir, Path::new("/p/examples"));
    assert_eq!(paths.attribution_dir, Path::new("/p/.napl/attribution"));
    assert_eq!(paths.ml_dir, Path::new("/p/.napl/mapl"));
}
