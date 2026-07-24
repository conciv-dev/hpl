//! On-disk state readers/writers: map, journal, lock. The filesystem I/O and the
//! parsing/serialization it wraps are the generated `state_io` crate; this shell
//! maps its message strings to `CliError` and preserves the public signatures.

use std::path::Path;

use napl_core::schemas::{HlLock, JournalEntry, NaplMap};

use crate::error::{CliError, CliResult};

pub use state_io::default_lock;

/// Read the map, or an empty map when absent, mirroring `readMap`.
pub fn read_map(map_path: &Path) -> CliResult<NaplMap> {
    state_io::read_map(map_path).map_err(CliError::new)
}

/// Write the map pretty-printed with a trailing newline, mirroring `writeMap`.
pub fn write_map(map_path: &Path, map: &NaplMap) -> CliResult<()> {
    state_io::write_map(map_path, map).map_err(CliError::new)
}

/// Read the journal, returning valid entries and skip-warnings, mirroring
/// `readJournal` (the caller decides how to surface warnings).
pub fn read_journal(journal_path: &Path) -> CliResult<(Vec<JournalEntry>, Vec<String>)> {
    state_io::read_journal(journal_path).map_err(CliError::new)
}

/// Append one compact JSON journal line, mirroring `appendJournalEntry`.
pub fn append_journal_entry(journal_path: &Path, entry: &JournalEntry) -> CliResult<()> {
    state_io::append_journal_entry(journal_path, entry).map_err(CliError::new)
}

/// Read and validate the lock, mirroring `readLock`.
pub fn read_lock(lock_path: &Path) -> CliResult<HlLock> {
    state_io::read_lock(lock_path).map_err(CliError::new)
}

/// Write the lock pretty-printed with a trailing newline, mirroring `writeLock`.
pub fn write_lock(lock_path: &Path, lock: &HlLock) -> CliResult<()> {
    state_io::write_lock(lock_path, lock).map_err(CliError::new)
}

/// Resolve prompt aliases from the lock, falling back to defaults, mirroring
/// `loadPromptAliases`.
pub fn load_prompt_aliases(lock_path: &Path) -> Vec<String> {
    state_io::load_prompt_aliases(lock_path)
}
