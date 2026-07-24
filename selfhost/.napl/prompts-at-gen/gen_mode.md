# Full-vs-incremental mode selection

This module is the **pure** decision core the code-generation command uses to
choose between a from-scratch (full) regeneration and an incremental update, and
to render the one-line `mode:` status it prints for each module. It is pure — no
filesystem access, no process spawning, no dependencies on other project modules.
The I/O shell reads the map record off disk, computes the primitive facts below,
consults this predicate, and prints the rendered message line verbatim.

## Where this code lives

The working directory is a Cargo workspace whose root manifest is written and
owned by the toolchain — leave it alone. Create this module as its own member
crate in a subdirectory named `gen_mode/`: `gen_mode/Cargo.toml` (package name
`gen_mode`) and `gen_mode/src/lib.rs`. Touch nothing outside `gen_mode/`. Ensure
`cargo test` passes from the workspace root before finishing.

## `can_incremental(full: bool, has_target_record: bool, unattributed_is_true: bool, has_prompt_hash_at_gen: bool) -> bool`

Decide whether an incremental update is possible. It is possible exactly when
**all** of these hold: the generation is not forced to full (`full` is `false`),
there is a prior target record in the map (`has_target_record` is `true`), the
target is not flagged unattributed (`unattributed_is_true` is `false`), and a prior
prompt hash was recorded at the last gen (`has_prompt_hash_at_gen` is `true`).
Equivalently: return `!full && has_target_record && !unattributed_is_true &&
has_prompt_hash_at_gen`. If any one condition fails, return `false`.

## The full-mode reasons

Expose a public enum with exactly three variants naming why a full (non-incremental)
generation was chosen: one for "there was no prior prompt body or attribution on
disk to diff against", one for "the caller forced a full generation with `--full`",
and one for "there was no prior successful gen for this target". Name the variants
clearly (for example `NoPriorOnDisk`, `ForcedFull`, `NoPriorGen`).

## `full_mode_message(reason) -> String`

Given a full-mode reason, return exactly the corresponding status line (each begins
with two leading spaces):

- the no-prior-on-disk reason →
  `  mode: full (no prior prompt body or attribution on disk to diff against)`
- the forced-full reason → `  mode: full (forced --full)`
- the no-prior-gen reason → `  mode: full (no prior successful gen for this target)`

## `incremental_mode_message(changed_lines: usize, affected_regions: usize) -> String`

Render the incremental status line with the two counts substituted. It begins with
two leading spaces and uses an em dash (`—`, U+2014):

`  mode: INCREMENTAL — <changed_lines> changed prompt line(s), <affected_regions> owned region(s) affected`

So `incremental_mode_message(3, 2)` is exactly
`  mode: INCREMENTAL — 3 changed prompt line(s), 2 owned region(s) affected`.
