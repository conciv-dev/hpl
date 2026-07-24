//! Generated-file healing: git-style move detection. A generated file the map
//! tracks can vanish from its recorded path because a hand `mv` (or a rename in
//! an editor) relocated it. Rather than reporting a false "deleted +
//! unattributed", the toolchain heals: it finds the untracked file that carries
//! the moved content, rewrites the path in the map, journals a `move` entry, and
//! relocks it at the new path. An exact content-hash match is a clean heal
//! (status stays clean); a line-similar match is a moved-and-drifted file (the
//! path is healed, then the normal drift machinery reports the content change).
//! Two candidates with the same hash are ambiguous, a hard error, never a
//! guess.
//!
//! Stage1: the pure move-match verdict (`lcs_len`, the clean/drifted/ambiguous
//! decision) is the NAPL-generated `healing_core` crate, and the filesystem walk
//! plus journal write is the NAPL-generated `healing_io` crate. Both are composed
//! here behind the unchanged public surface; the clock seam (`now`) is injected
//! from the hand-written `clock` shell.

use std::path::Path;

use napl_core::schemas::{JournalEntry, NaplMap};

use crate::error::{CliError, CliResult};
use crate::paths::NaplPaths;

pub use healing_io::HealedMove;

/// Heal every tracked file the map lost to a move, mutating `map` and appending
/// one `move` journal entry per heal. Returns the heals applied (empty when
/// nothing moved). The caller persists the mutated map.
pub fn heal_moved_files(
    root: &Path,
    paths: &NaplPaths,
    map: &mut NaplMap,
    journal: &[JournalEntry],
) -> CliResult<Vec<HealedMove>> {
    healing_io::heal_moved_files(root, paths, map, journal, &crate::clock::now)
        .map_err(CliError::new)
}
