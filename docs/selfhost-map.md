# Self-Host Module Map — the campaign tracker

This is the dependency-ordered inventory of the toolchain being re-expressed as
`.napl` modules. It is the tracker for the self-host campaign: every module the
stage0 binary must eventually regenerate from prose, ordered so a generated
module is only depended on by a later gen once it has passed its equivalence
gate.

The unit of self-hosting is a **module** (`rust/crates/napl-core/src/<m>.rs` or
`schemas/<m>.rs`). Each module's hand-written `#[cfg(test)] mod tests` suite is
its **equivalence corpus** — the exact input→output cases the generated code must
reproduce (behaviorally, not byte- or type-identically).

## The pure/IO split decides the phase order

`napl-core` is **entirely pure**: no module under `napl-core/src/` touches
`std::fs`, `std::process`, `std::io`, or `std::env`. It is deterministic
data-in/data-out logic, which is exactly what the behavioral-equivalence gate can
prove. This is why the whole self-host campaign starts and lives in `napl-core`.

All **I/O lives one crate up**, in `napl-cli` (`cmd_*`, `fsutil`, `process`,
`paths`, `clock`) and in `napl-lsp` (the tower-lsp server). Those modules drive
filesystems, subprocesses, and a JSON-RPC loop; their "corpus" is the conformance
suite and the LSP integration tests, not per-function unit vectors. They are a
**later phase** and are inventoried here only for completeness.

## Stage1 swap-in status (SHIPPING)

Phase 1 is not just self-hosted in the harness — it is **swapped in**: the shipping
`napl` binary now runs the generated crates. `napl-core`'s hand-written module
bodies are deleted and replaced by thin adapters over the generated crates
(cross-workspace path-deps; `selfhost/` untouched). **All 23 of 23 modules run
generated code; conformance is 40/40 byte-identical; the escape-hatch list is
empty.**

- **`schemas::journal` — escape-hatch CLEARED.** The generated `read_journal_str`
  previously emitted corrupt-line warning text (`"line 2: expected ident …"`)
  diverging from what conformance `34-journal-corrupt-line` pins byte-for-byte
  (`journal: skipping corrupt line 2 (invalid JSON)`). The `schemas_journal.napl`
  prompt now pins that exact warning as behavior prose plus a byte-exact given/
  expect case, and describes the two-phase parse (arbitrary-JSON syntax check
  first → `(invalid JSON)`; deserialize/validation failure second → the same
  `journal: skipping corrupt line {n} ` prefix). Re-genned on **attempt 1/3**, the
  generated crate now produces the pinned bytes, so `schemas::journal` is a
  straight re-export adapter (no error-message seam) and ships. The equivalence
  gate was extended to assert the warning **text** (9/9, was 8/8).
  (`schemas::frontmatter` remains bridged — its `FrontmatterError` is mapped to the
  pinned message text in the adapter, because the generated `prompts` crate
  composes on `schemas_frontmatter::Frontmatter`.)

See `selfhost.md` → "Stage1 swap-in — DONE" and "Journal escape-hatch cleared +
Phase 2 opened" for the workspace-membership call, the adapter seam catalog, and
the full gate numbers.

## Phase 1 — `napl-core` (pure; the active campaign)

Waves are dependency layers. Wave 1 = pure leaves with **no intra-crate
dependency**. Wave *n* depends only on waves `< n`. Every `napl-core` module is
pure, so the "pure/IO" axis is uniform here; the differentiator is dependency
depth and corpus size.

### Wave 1 — pure leaves (no intra-crate deps)

| Module | LOC | Unit tests | External crates | Self-host status |
| --- | ---: | ---: | --- | --- |
| `body_lines` | 111 | 5 | — | **done** (pilot, re-genned as workspace member in slice 2, 5/5) |
| `extensions` | 141 | 7 | — | **done** (slice 1) |
| `hash` | 44 | 4 | `sha2`, `hex` | **done** (slice 1) |
| `parse_output` | 39 | 3 | — | **done** (slice 1) |
| `text_diff` | 392 | 11 | — | **done** (slice 1) |
| `drift` | 184 | 3 | — | **done** (slice 2, 3/3) |
| `scanner` | 634 | 12 | — | **done** (slice 2, 12/12) |
| `targets` | 272 | 6 | — | **done** (slice 2, 9/9 — corpus grew with the workspace fields) |
| `guard` | 189 | 5 | `serde` | **done** (slice 2, 5/5) |
| `schemas::frontmatter` | 180 | 6 | — | **done** (slice 2, 6/6) |
| `schemas::ir` | 123 | 6 | — | **done** (slice 2, 6/6) |
| `schemas::line_range` | 159 | 8 | — | **done** (slice 2, 8/8) |
| `schemas::ordered_map` | 163 | 4 | — | **done** (slice 2, 4/4) |

Wave 1 is fully self-hosted: **13/13 modules**, 83 equivalence cases green.

Waves 1–3 together: **23/23 modules, 189 equivalence cases green**, escape-hatch
list still empty — all of `napl-core` self-hosts (phase 1 complete).

### Wave 2 — depends only on wave 1

Wave 2 is fully self-hosted: **6/6 modules, 72 equivalence cases green** (slice 3),
each generated on attempt 1 of 3. Every generated wave-2 crate **path-deps the
generated wave-1 crate(s)** it builds on — it composes on the generated code, not
on hand-written napl-core.

| Module | LOC | Unit tests | Intra-crate deps | Self-host status |
| --- | ---: | ---: | --- | --- |
| `blame` | 303 | 13 | `text_diff` | **done** (slice 3, 13/13 — path-deps generated `text_diff`) |
| `reverse` | 297 | 12 | `body_lines`, `schemas` | **done** (slice 3, 12/12 — path-deps generated `body_lines` + `schemas_attribution` + `schemas_line_range`) |
| `schemas::attribution` | 173 | 9 | `line_range` | **done** (slice 3, 9/9 — path-deps generated `schemas_line_range`) |
| `schemas::lock` | 290 | 19 | `extensions` | **done** (slice 3, 20/20 — path-deps generated `extensions`; +1 empty-model case) |
| `schemas::map` | 553 | 10 | `ordered_map` | **done** (slice 3, 10/10 — path-deps generated `schemas_ordered_map`) |
| `schemas::ml` | 185 | 8 | `line_range` | **done** (slice 3, 8/8 — path-deps generated `schemas_line_range`) |

`blame` was confirmed to depend **only** on `text_diff`: its `BlameSourceEntry`
is a blame-local struct, not a `schemas::journal` type, so no wave-3 journal
pull-forward was needed. (`schemas::journal` depends on `blame`, not the reverse.)

### Wave 3 — aggregates over waves 1–2

Wave 3 is fully self-hosted: **4/4 modules, 34 equivalence cases green** (slice 4),
each generated on **attempt 1 of 3**. Every generated wave-3 crate path-deps the
generated wave-1/2 crate(s) it builds on.

| Module | LOC | Unit tests | Builds on (generated crate) | Self-host status |
| --- | ---: | ---: | --- | --- |
| `schemas::journal` | 228 | 8 | `blame`, `text_diff` | **done** (slice 4, 8/8) — **escape-hatch at stage1 swap-in** (warning-text divergence; hand-written body ships) |
| `incremental` | 235 | 2 | `schemas_attribution`, `schemas_line_range` | **done** (slice 4, 3/3 — 2 corpus + 1 composition case) |
| `yaml` | 535 | 9 | `schemas_attribution`, `schemas_ir`, `schemas_ml`, `schemas_line_range` | **done** (slice 4, 9/9 — byte-exact block goldens) |
| `prompts` | 523 | 7 | `schemas_attribution`, `schemas_frontmatter`, `schemas_line_range`, `targets` | **done** (slice 4, 14/14 — 7 corpus + 7 byte-exact pins) |

`lib.rs` (23 LOC, 0 tests) is a pure re-export root and is not a self-host unit.

**Phase 1 (`napl-core`) is COMPLETE: 23/23 modules, 190 equivalence cases green,
every module converged on attempt 1** — and now **FULLY SWAPPED IN**: the shipping
binary runs generated code for all 23 of 23 modules, conformance 40/40
byte-identical, escape-hatch list empty (`schemas::journal` cleared — see the
stage1 status section above). See `selfhost.md` → "Stage1 swap-in — DONE" and
"Journal escape-hatch cleared + Phase 2 opened".

## Phase 2 — `napl-cli` (I/O orchestration; OPEN, first batch swapped in)

Phase 2 is not behavioral-unit self-hostable the way `napl-core` is — the command
handlers are gated by the **conformance corpus** (`conformance/`, 40 scenarios),
not per-function vectors. But most I/O modules wrap a **pure core** that can be
extracted, generated, and gated by that module's existing pure unit tests. The
discipline is the campaign's: **extract-pure-core, keep-thin-I/O-shell** — split
each module so the deterministic data-in/data-out logic lives in a function the
generated crate can replace, keep the filesystem/subprocess plumbing hand-written
in the shell, and swap the generated pure core in behind the shell's call sites.

**Gate strategy.** The conformance corpus is the behavioral spec for the `cmd_*`
handlers (the fake-agent harness makes gens deterministic); the per-module pure
unit corpora are the equivalence gate for the extracted cores. A pure-core
extraction refactor must keep conformance byte-identical **before** the generated
swap; then the swap must keep it byte-identical again.

### Per-module pure/IO split (the plan)

| Module | LOC | Pure core (self-host unit) | I/O shell (stays hand-written) | Pure unit tests | Status |
| --- | ---: | --- | --- | ---: | --- |
| `clock` | 65 | `iso_from_millis` (millis → ISO-8601) + civil-date math | `now()` (reads wall clock / `NAPL_FIXED_NOW`) | 3 | **swapped** (`clock_fmt`, batch 1) |
| `paths` | 126 | `resolve_paths` + `NaplPaths` + `rel_to` (path algebra) | `find_prompt_files`/`walk` (readdir) | 1 (`rel_to`) | **swapped** (`paths_core`, batch 1) |
| `statusclass` | 213 | `FileStatus` + `StatusEntry` + `line`/`is_error` (render) | `classify_prompt`/`detect_drift` (fs read + hash) | 2 | **swapped** (`statusclass_render`, batch 1) |
| `driftdetect` | 146 | `reconstruct_file_content` (journal patch replay) | `classify_file`/`detect_gen_drift` (fs read) | 2 | **swapped** (`driftdetect_replay`, batch 1 — composes on generated `schemas_journal` + `text_diff`) |
| `snapshot` | 147 | `diff_snapshots` (before/after hash diff) | `walk`/`snapshot_hashes`/`snapshot_contents` + `make_filter`/`SnapshotFilter` | 1 (`diff_snapshots`) | **swapped** (`snapshot_diff`, batch 1) |
| `fsutil` | 70 | — (only the mode constants are pure; every fn is fs I/O) | all (`read_opt`/`write`/`set_mode`/`exists`/`mkdir_parent`) | 0 pure | **shell** (no pure slice with a unit test) |
| `error` | 35 | msg-extraction from `SchemaError` | type + `From` trait glue over hand-written `SchemaError`/`io::Error` | 0 | **shell** (inseparable from caller types) |
| `process` | 435 | — | subprocess spawn + lockfile (all 4 tests are fs I/O) | 0 pure | **shell** |
| `state` | 89 | in-memory state | — | 0 | **shell** |
| `driftdetect`/`snapshot` filter slices | — | `make_filter`/`SnapshotFilter::is_excluded_file` are pure but have no direct unit test | walk uses them | 0 pure | later batch (add a pure unit test first) |
| `cmd_*` (gen/status/init/watch/reconcile/blame/build/test) | ~1900 | derivation/orchestration | I/O + orchestration | 0 | **shell** (conformance-gated) |
| `main` | 184 | — | arg parsing / dispatch | 0 | **shell** |

`cmd_gen` (1133 LOC) is the stage0 orchestrator itself — the last thing to
self-host, and the true fixpoint when it does. The shells shrink as more pure cores
are extracted; a module is only "shell" until its next pure slice grows a unit test.

### Batch 1 — the low-risk leaves (DONE, all swapped in)

Five pure cores generated from behavior-prose prompts in `selfhost/`, each
converged on **attempt 1 of 3**, each gated by that module's exact hand-written
pure unit corpus in the shared equivalence harness, each swapped into `napl-cli`
behind its existing call sites (thin re-export; hand-written pure body deleted;
the unit corpus rides along as the regression net):

| Generated crate | Replaces (napl-cli module's pure slice) | Deps | Equivalence |
| --- | --- | --- | --- |
| `clock_fmt` | `clock::iso_from_millis` | — | 3/3 (byte-exact ISO strings) |
| `paths_core` | `paths::{resolve_paths, NaplPaths, rel_to}` | — | 2/2 (rel_to + full layout) |
| `statusclass_render` | `statusclass::{FileStatus, StatusEntry, line, is_error}` | — | 2/2 (byte-exact status lines) |
| `driftdetect_replay` | `driftdetect::reconstruct_file_content` | `schemas_journal`, `text_diff` | 2/2 (composes on generated phase-1 crates) |
| `snapshot_diff` | `snapshot::diff_snapshots` | — | 1/1 |

**Batch-1 evidence:** `driftdetect_replay` is the notable one — a phase-2 pure core
composing on **generated phase-1** crates by path (`schemas_journal::JournalEntry`
inputs, `text_diff::{parse_hunks, apply_hunks}` replay). Because napl-core already
re-exports `schemas_journal::JournalEntry` (JOB A) and both crates path-dep the same
`schemas_journal`, the types unify and the shell passes `&[napl_core::schemas::
JournalEntry]` straight through. No extraction refactor was needed: each pure core
was already a cleanly separable function/type in its module, so the swap is a
re-export behind the unchanged call sites, and conformance stayed 40/40
byte-identical across every swap.

### Batch 2 candidates (recommended next)

The next low-risk pure slices, in rough order: the `snapshot`/`SnapshotFilter`
exclusion predicate (`make_filter` + `is_excluded_file` — pure, but needs a direct
unit test added first), then the pure derivation helpers inside `cmd_reconcile` /
`cmd_blame` that operate over already-read journal/map data. The `cmd_*` handlers,
`process`, `fsutil`, `error`, `state`, and `main` stay hand-written shells until
their pure cores grow (or until the conformance-gated `cmd_gen` fixpoint work).

## Phase 3 — `napl-lsp` (JSON-RPC server; later)

Gated by the LSP `integration` suite (12 tests). `classify`, `ml`, `convert`,
`context` are pure enough to be pulled forward; `backend`/`navigation`/`hover`/
`diagnostics` are server plumbing.

| Module | LOC | Unit/integration tests | Character |
| --- | ---: | ---: | --- |
| `integration` | 154 | 12 | end-to-end LSP corpus |
| `ml` | 144 | 4 | pure machine-layer parsing |
| `hover` / `navigation` / `diagnostics` / `backend` / `context` / `classify` / `convert` / `state` / `testkit` / `lib` | ~1500 | 0 | server plumbing |

## Fixpoint definition

The toolchain self-hosts when, for every non-escape-hatch module, stage0-generated
code passes that module's hand-written unit corpus under the shared equivalence
harness. Byte-identity is never required. The campaign is complete when the
generated toolchain can regenerate itself and still pass every corpus.

## Escape-hatch list

Modules that stay hand-written because current stage0 + prompt cannot reproduce
their behavior under the equivalence gate. A module leaves the list only when its
prompt drives a passing generation.

- **Generation:** *(empty)* — no module (phase 1 or phase-2 batch 1) has failed to
  converge (28/28 generated modules on attempt 1).
- **Stage1 swap-in:** *(empty)* — `schemas::journal` was **cleared**: the prompt now
  pins the byte-exact corrupt-line warning (`journal: skipping corrupt line {n}
  (invalid JSON)`) as behavior prose + a given/expect case, the crate re-genned on
  attempt 1 to produce those bytes, and the equivalence gate now asserts the warning
  text (9/9). All 23 `napl-core` modules ship generated code with no seam left on
  the hatch.
- **Phase 2 swap-in:** *(empty)* — batch-1's five pure cores all swapped in with
  conformance 40/40 byte-identical.

## Layout note (RESOLVED in slice 2 — Cargo workspace)

The stage0 gen target dir is fixed at `.napl/src/rust/` per target, and generated
files are locked `0444`. The pilot put `body_lines` **at the crate root**, and
slice 1 landed later modules as **nested package crates** in subdirectories — a
layout that composed only because Cargo silently ignores a nested package from the
root, which also meant the in-gen `cargo test` gate covered only the root crate.

Slice 2 replaced this with a real **Cargo workspace**. The rust target adapter now
carries `workspace_layout = true` and an `attribution_exclude_root_files` list;
the toolchain writes and owns the workspace root `Cargo.toml` (a virtual
`[workspace]` manifest listing every member crate), refreshing it on each gen so a
new module is added as a member before its `cargo test` runs. The root manifest is
treated like the guard files (AGENTS.md/CLAUDE.md): toolchain-owned, excluded from
attribution (root-level only — per-module `Cargo.toml` files stay attributed), not
locked, not drift-checked. Every module — `body_lines` included — is now a uniform
member crate at `.napl/src/rust/<module>/`, and the in-gen `cargo test` runs at the
workspace root, covering **all 13 members** in one gate. The shared equivalence
harness (`selfhost/equivalence/`) remains the behavioral cross-module gate.
</content>
</invoke>
