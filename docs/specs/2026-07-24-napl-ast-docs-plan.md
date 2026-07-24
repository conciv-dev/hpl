# NAPL AST, docs-from-source, and doc-comment injection: implementation plan

Date: 2026-07-24. Status: plan for the design at `docs/specs/2026-07-24-napl-ast-docs-design.md` (rev 5, committed). That design is the sole source of truth for behavior; this document is the sequencing and mechanics of building it. Where the two ever disagree, the design wins and this plan is wrong.

## How to read this plan

The design's Part 4 defines five migration slices A through E. Slices A, B, C, and E each fit one gen cadence and are kept whole. Slice D (injection-ON) is too large for a single gen cycle and is split into D1 through D4 below, ordered so each sub-slice has a green corpus before the next begins. Every slice is expressed in the standing prompt-only loop (next section). Pure algorithm modules and I/O shell modules are BOTH prompt-authored: pure modules live under `selfhost/core/` and `selfhost/schemas/`, shell modules under `selfhost/cli/` (the shell wave already prompted `state_io`, `paths_walk`, `statusclass_io`, `snapshot_io`, `driftdetect_io`, `process_run`, and the rest of the cli layer). This plan creates ZERO hand-written PRODUCTION Rust (the scope section below defines the claim; equivalence test files are hand-authored spec artifacts). Where a slice says "shell", it means a prompt edit to an existing `selfhost/cli/*.napl` shell module, or a new shell prompt, followed by a regen. The plan still names pure and shell work separately per slice because their prompts, gates, and blast radii differ.

## Prerequisites

This plan starts only after the self-host campaign's shell wave AND lsp wave complete and `rust-final` is tagged: every `cmd_*` command shell, `healing`, `discovery`, the `main`/`error`/`clock` glue, and the lsp modules must already exist as prompted modules, because Slices A, C, D2, D3, D4, and E all edit shell prompts and Slice A edits lsp reader prompts. Two specific dependencies: the `cmd_gen` skeleton prompt (shell wave batch 5) is the direct substrate for D2's pipeline work, and the `healing_io` module (pending its pure-LCS extraction decision) is the substrate for D3's metadata rewrite. Starting any slice before its substrate prompt exists would force hand-written Rust and violate the prompt-only mandate.

Checkpoint 0 (prerequisite audit, blocking): the shell and lsp waves will settle module names, public APIs, and dependency edges that this plan can only anticipate (`cmd_gen`, `cmd_status`, `cmd_init`, `cmd_blame`, `healing_io`, `main` glue do not exist as prompts today). Before Slice A starts, audit the landed prompt inventory against every module name this plan references, rewrite the affected work items and estimates, and re-review the deltas. Until that audit, the per-slice edit lists for shell modules are provisional.

Runner extension (pre-slice, blocking for scenarios `93`, `102`, `105`, `108`): the conformance runner supports exactly one `run: string[]` per scenario, so multi-phase assertions (status then gen, double gen, recovery idempotence, reconcile then regen) are not expressible today. Extend the runner (TS, `conformance/runner/`) with an optional `runs:` list of `{run, expect}` phases sharing one tree, additive to the existing single-`run` shape. Gate the extension itself by re-running the FULL existing corpus byte-identical (no existing scenario changes shape). This is harness code, not language behavior; the corpus stays the sole spec of the CLI.

## Scope of the prompt-only claim

Prompt-only means zero hand-written PRODUCTION Rust in the toolchain: every pure module and every I/O shell module is prompt-authored and generated. Two artifact classes are hand-authored by design and are not exceptions but spec surfaces: the equivalence corpus (hand-written Rust test files under `selfhost/equivalence/`, they ARE the behavioral spec) and conformance scenarios with their goldens. TS surfaces outside the toolchain (`packages/`, `apps/site`) remain hand-written; the one this plan names is the `byteToUtf16` editor helper in Slice B.

## The standing prompt-only loop (applied to every pure module in every slice)

This feature is the first authored prompt-only end to end. The design nominates it; this plan operationalizes it. For each new or changed pure module the loop is fixed and non-negotiable:

1. Author or extend the module's equivalence corpus FIRST, as a hand-written Rust test file under `selfhost/equivalence/tests/<module>.rs`, asserting exact input to output including byte-exact spans and byte-exact serialized output where relevant. The corpus is the behavioral spec; it exists before the prompt claims to satisfy it.
2. Author or extend the conformance scenarios that exercise the module through the CLI (Slice-level, byte-identical goldens). Scenarios are the sole spec of CLI behavior; goldens are written by running `dump` and sanctioning the bytes, never hand-typed.
3. Write or extend the `.napl` prompt under `selfhost/<layer>/<module>.napl` with `deps:` frontmatter and `tests:` given/expect cases, specifying behavior at the spec level (what, with worked examples and pinned strings), never transcribing an implementation.
4. Run `napl gen` for that module, strictly foreground, one module at a time, never backgrounded, never the pnpm wrapper. The coding agent generates the Rust into `selfhost/.napl/src/rust/<module>/` (locked 0444).
5. Run the slice gate battery (below). Done means: equivalence green, conformance byte-identical, `cargo test --workspace` green in `rust/`, `cargo clippy --workspace --all-targets` clean, generated-tree `cargo test` green, `napl status` fully clean.

Every regen of an EXISTING module is classified up front, before it runs, as one of exactly two kinds, and the slice text below names the kind for each edit:

- Expected no-op: the prompt edit is not meant to change semantics (a trim, a restatement). The regen must produce ZERO file patches with an honest mapl no-op entry. Any patch halts the slice for investigation.
- Expected change: the prompt edit adds or alters behavior. The regen MUST produce patches, and the plan enumerates the anticipated patch scope (which files, which behavior). Every patch line is read; any patch outside the enumerated scope halts the slice. Sanctioned conformance-golden edits are enumerated per slice the same way.

Mixing the two classes on one edit is forbidden: an edit that both restates prose and adds behavior is split into two edits, no-op first.

Escape-hatch discipline: a module that will not converge in three gen attempts is recorded in `docs/selfhost-map.md` with its reason and bridged in the shell adapter; convergence is never forced.

## Gate battery (the exact commands, referenced by every slice as "the gates")

- Conformance: `cd conformance && node --experimental-strip-types runner/cli.ts` green, and per-scenario `node --experimental-strip-types runner/cli.ts dump <name>` used to sanction goldens.
- Equivalence: `cd selfhost/equivalence && cargo test` green.
- Workspace unit tests: `cd rust && cargo test --workspace` green.
- Lint: `cd rust && cargo clippy --workspace --all-targets` clean.
- Generated tree: `cargo test` inside the generated workspace under `selfhost/.napl/src/rust/` green.
- Sync: `napl status` reports fully clean (no drift, no staleness, no interrupted marker, no orphan notes).
- Root JS surfaces touched: `pnpm format` and `pnpm lint` at the root before finishing.

A slice is "done" only when every listed gate is green with no sanctioned golden edit beyond the ones the slice enumerates.

Rollback baseline (standing, applies to every slice): each slice STARTS from a committed clean tree, and its first act is recording a state manifest to the slice log: the start commit hash, the journal byte length, the locked-crate inventory with hashes, and the map hash. "Restore from git" in the per-slice rollback stories means restoring against THIS manifest: prompt revert, locked-crate restore, journal truncation to the recorded byte length, and map/attribution restore, in that order, with surgical edits (never wholesale checkout) for any shared state file that later slices or concurrent campaign work also touched. A rollback that cannot be expressed against the manifest is not performed; the slice halts for human review instead.

---

## Slice A: attribution v2 and target scoping (design Part 0)

### Goal and scope

Implements design Part 0 in full: the target-scoped attribution layout `.napl/attribution/<module>/<target>.yaml` with document `version: 2`, dual-layout readers, per-(module,target) precedence, migrate-on-gen with scoped v1 deletion, and orphan-note reporting. This is a prerequisite for every later slice because injection writes per-file display records into the v2 document and multi-target modules must stop clobbering. No generated ARTIFACT bytes change; only attribution state paths and contents change, exactly as the scenarios enumerate.

### New and changed conformance scenarios

- `90-attribution-v1-read`: setup seeds an old-path `.napl/attribution/<m>.yaml` (no version field) and a matching map; `napl status` on the untouched tree reads it and reports clean. Asserts the v1 reader path still resolves.
- `91-attribution-v2-migrate-on-gen`: setup seeds a v1 attribution file whose `target` equals the target about to be genned; a normal gen writes `.napl/attribution/<m>/<target>.yaml` with `version: 2` and DELETES the v1 file. Fixture shape: identical greeting module to `10-gen-happy`, but the expected files block asserts the new nested path present with `version: 2` and the old flat path `absent: true`.
- `92-attribution-multi-target-no-clobber`: a two-target module (typescript and rust) where each target's v2 document coexists under its own `<target>.yaml`; gen of one target leaves the other target's v2 file byte-untouched. Asserts both nested files present with distinct content after a single-target gen.
- `93-attribution-orphan-note` (multi-phase via the runner extension): a module whose frontmatter drops a previously-genned target; `napl status` reports the now-orphaned `<target>.yaml` as a note (exit 0, note text pinned), and the next gen cleans it. Phase one runs status and asserts the note string; phase two runs gen and asserts the orphan file `absent: true`.
- Precedence sub-case inside `92`: a v1 file present alongside the v2 file for the same (module,target) is ignored; the v2 document is authoritative. Asserted by seeding both and reading the v2 content.
- Retention sub-case inside `92`: a v1 file whose `target` differs from the generated target is IGNORED for the generated target and RETAINED on disk (its own target's gen migrates it later). Asserted by seeding a v1 file for the other target, genning one target, and asserting the v1 file byte-untouched.

Sanctioned golden edits: the attribution-path fields in `91`, `92`, `93` goldens change from flat to nested and gain `version: 2`; no map.json artifact hash changes; enumerate each changed path in the slice's golden-edit ledger.

### New and changed .napl prompts

- `selfhost/schemas/schemas_attribution.napl` (extend). Add `version: 2` to the document type (optional, defaulting to a v1-implied 1 when absent so v1 documents still deserialize), and add the per-file display-metadata record type the later slices populate (`{path, undocumented_hash, documented_hash, blocks: [...]}`), defaulting to empty so Slice A writes none, plus the optional load-bearing `injectedGen` scalar (absent default, first written in D2, consumed by D4's activation signal). Behavior prose specifies: v2 documents carry `version: 2`; absent version means legacy v1; the reader accepts both. Regen class: EXPECTED CHANGE scoped to the `schemas_attribution` crate itself (new types, extended parser and serializer) with ZERO byte change to any other module's generated artifacts; serializer field order and default-omission rules pinned in prose so the patch stays additive. Extend `tests:` with a v1-read case and a v2-with-version case before genning.
- Transition readers (required by the design: every consumer accepts both layouts during migration). Three additional work items, all regen class expected change scoped to attribution path resolution: (a) the lsp attribution-reader prompt gains dual-layout resolution, (b) the wasm binding's attribution access gains the same (prompted module if the wasm surface is prompted by then, else the TS/wasm glue, hand-edited as a named non-toolchain surface), (c) the site fixture builder (`apps/site`, TS) reads both layouts. Each gets a reader test against a seeded v1 tree and a seeded v2 tree. Slice A is not done while any consumer reads only one layout.
- Path resolution, precedence, migrate-on-gen deletion, orphan detection, and the orphan-note text live in the shell modules: prompt edits to `selfhost/cli/state_io.napl` and `selfhost/cli/paths_walk.napl` (attribution path layout), and to the `cmd_gen` and `cmd_status` shell prompts (migrate-on-gen, orphan note), each followed by its regen and gated by scenarios `90` through `93`. No new pure module is required for Slice A beyond the schema extension; the layout decision is filesystem policy, which is shell, which is still a prompt.

### State and schema migrations

- New on-disk layout `.napl/attribution/<module>/<target>.yaml`; old `.napl/attribution/<module>.yaml` remains readable. Backward-compat: a v1-only tree reads clean; a v1 binary reading a v2 tree fails its strict parse loudly (documented, acceptable pre-1.0). Migration byte-identity claim is scoped exactly per design r2 finding 14: generated artifacts stay byte-identical, attribution state paths and contents change as the scenarios enumerate.
- Refusal behavior: none new; gen migrates, status notes-and-does-not-touch.

### Gates

The gate battery. Additionally: the `schemas_attribution` regen is expected-change scoped to its own crate; diff every OTHER module's generated artifact to zero. Confirm the existing 45 modules' attribution files migrate on their next gen without artifact change by regenerating two representative multi-target modules and diffing artifacts to zero. Run the three transition-reader tests (lsp, wasm, site fixture builder) against seeded v1 and v2 trees.

### Rollback story

Schema change is additive and default-guarded; if the `schemas_attribution` regen produces patches OUTSIDE its enumerated scope (its own crate's new types, parser, serializer), revert the prompt edit against the slice manifest, restore the prior locked crate, and re-derive the prose. Shell path-migration bugs are caught by scenarios `91` through `93` before any real tree migrates; the shell edit is revertible in isolation because no pure crate depends on it.

### Estimated gen cycles

8. Two for the `schemas_attribution` extension (a possible second attempt if key-order pinning slips), four shell regens (`state_io`, `paths_walk`, `cmd_gen`, `cmd_status`), and two reader regens (lsp attribution reader, wasm attribution access), each an expected change with enumerated scope read line by line. The site fixture builder is TS and not a gen cycle.

---

## Slice B: the NAPL AST parser and `napl parse --json` (design Part 1)

### Goal and scope

Implements design Part 1: a read-only AST over prompt documents, `astVersion: 1`, pinned grammar, raw-byte UTF-8 end-exclusive offsets plus 1-based inclusive lines, `problems` list, BOM and CRLF handling, and the wasm-facing 64 MiB input guard. Adds `napl parse --json` as a debug subcommand emitting the AST as the fixture contract LSP and site consume. `SentenceRange` is included as a display-only query layer; nothing in gen or status consults it.

### New and changed conformance scenarios

- `94-parse-json-basic`: `napl parse --json <file>` on a small frontmatter-plus-headings-plus-list document; asserts the full serialized AST byte-for-byte, including `astVersion: 1`, each block's `byte_start`/`byte_end` (end-exclusive) and `line_start`/`line_end` (1-based inclusive).
- `95-parse-json-crlf`: a CRLF document; asserts `\r` is included in byte ranges and lines behave identically to LF for line numbers. Fixture: same body as `94` with `\r\n` endings; goldens differ only in the byte offsets.
- `96-parse-json-no-frontmatter`: a malformed or absent frontmatter document; asserts `frontmatter: null`, body starting at byte 0, and a populated `problems` list, with parse still exiting 0.
- `97-parse-json-unclosed-fence`: an unclosed code fence runs to EOF as one `CodeFence` block; asserts the span reaches the final byte.
- `98-parse-json-literal-fallback`: a document containing a table, a block quote, and a link, all of which v1 does not recognize; asserts each is captured as literal `Paragraph` text and `astVersion` stays 1.

Fixture shape for all: `setup` writes one `.napl` file; `run: [parse, --json, <path>]`; `expect.stdout` is the exact serialized JSON. No agent, ir, attribution, or ml script blocks are needed (parse does no gen).

### New and changed .napl prompts

- `selfhost/core/ast_parse.napl` (new). `deps: []` (pure, scans raw bytes; must NOT compose on `body_lines`, which strips `\r` per design r2 finding 12). Behavior prose specifies: the pinned grammar (`Heading` atx levels 1-6, `Paragraph` with spans and sentences, `List` ordered or bullet with tight flag and depth at most 3, `CodeFence` with info and text and unclosed-to-EOF rule, `ThematicBreak`); the span span type (`byte_start`/`byte_end` end-exclusive u64, `line_start`/`line_end` 1-based inclusive); BOM retained-in-offsets-but-skipped-before-frontmatter-detection; CRLF `\r` belongs to its line; malformed frontmatter yields `frontmatter: null` and a `problems` entry; the constructs NOT recognized in v1 (block quotes, HTML, links, images, reference definitions, footnotes, tables, indented code, setext headings) fall through to literal paragraph text; grammar evolution bumps `astVersion`, so v1 output is pinned. Includes the `SentenceRange` display-only type and the line-intersection join rule, with prose stating nothing in gen or status consults it. Worked examples pin at least: a heading span, a fenced-block span, a CRLF byte range, and a no-frontmatter `problems` case. Expected-no-op risk: not applicable (new module).
- `napl parse --json` subcommand: new shell prompt `selfhost/cli/cmd_parse.napl` with `deps: [ast_parse]` plus a dispatch line in the `main` glue prompt. Behavior prose: read one file, call `ast_parse`, serialize via the pinned JSON writer discipline, print to stdout, exit codes. Gated by scenarios `94` through `98`.
- `@napl/editor` `byteToUtf16(text, offset)` helper (TS, hand-written in `packages/`): tests cover astral plane, CRLF, and mid-scalar clamp-and-flag. Out of the prompt-only loop (it is a TS surface, see the scope section), inside the slice's gate for `pnpm lint`/`pnpm format`. Named here for completeness; it consumes the AST offsets.
- wasm `parseDocument` binding: the design pins the wasm surface (parse a document from JS, `astVersion` pinned, inputs above 64 MiB rejected at the boundary). Work item: extend the wasm binding module (prompted if the wasm crate is prompted by then, else the binding glue as a named non-toolchain surface) to expose `parseDocument`, with wasm-side tests asserting `astVersion: 1`, the 64 MiB rejection error, and CRLF/BOM byte offsets THROUGH the JS-facing API (offsets are raw-byte, so JS consumers must receive them unmangled and convert via `byteToUtf16`). The 64 MiB guard lives at the wasm boundary, not inside `ast_parse`.
- LSP and site consumption of the AST are deferred to the AST/docs feature's consumers and are NOT Slice B work items; Slice B ships the parser, the CLI surface, and the wasm surface.

### State and schema migrations

None. The AST is a read-only view; it persists nothing and touches no existing schema. Output carries `astVersion: 1`; consumers pin it.

### Gates

The gate battery, with the equivalence corpus for `ast_parse` (spans, CRLF, BOM, problems, fence-to-EOF, literal fallbacks) authored first and green. Scenarios `94` through `98` byte-identical.

### Rollback story

`ast_parse` is a leaf with no dependents in this slice; if it will not converge, park it on the escape-hatch list and iterate the prose against the equivalence corpus. `cmd_parse` is an additive shell module; deleting its prompt and crate removes the subcommand without touching any pure crate or existing scenario.

### Estimated gen cycles

7. Two to three for `ast_parse` (grammar breadth: five block kinds, span duality, edge cases make one-shot convergence unlikely; corpus-driven prose tightening), one for the new `cmd_parse` shell module, one for the `main` glue dispatch regen, two for the wasm `parseDocument` binding (the boundary guard and offset-fidelity tests make a second attempt likely).

---

## Slice C: injection flag inertness (design Part 4 item 3)

### Goal and scope

Proves the feature is inert when off. Adds the `docs.inject` config surface to the lock schema (defaulting false, absent means false) and confirms the ENTIRE existing corpus, with `docs` absent or false, produces generated artifacts byte-identical to pre-feature goldens. State files change only per Slice A's enumeration. This slice lands the lock-schema change and the config plumbing without any injection behavior, isolating the schema change from the behavior change.

### New and changed conformance scenarios

- No new scenario asserts new behavior. Instead: the whole existing corpus is re-run and must stay byte-identical after the lock-schema module regen. This is the assertion.
- `99-docs-flag-absent-inert`: an explicit scenario documenting the guarantee: a gen with `docs` absent from `lock.json` behaves exactly as `10-gen-happy` (no header, no item docs, identical artifact bytes). Fixture is `10-gen-happy` with an assertion that no doc-form comment appears in the output file.
- `100-docs-flag-false-inert`: same, with `"docs": {"inject": false}` explicit in `lock.json`. Asserts identical artifact bytes and that the strict parser accepts the explicit-false form.

### New and changed .napl prompts

- `selfhost/schemas/schemas_lock.napl` (extend). Add the `docs` object with a strict-parsed boolean `inject`, absent meaning false, unknown keys inside `docs` rejected (matching the existing deny-unknown-fields discipline for `agent`). Behavior prose: `docs.inject` defaults false; `napl init` writes true (init text is shell, noted below). Add `tests:` cases: absent `docs` yields inject false; explicit false kept; explicit true kept; unknown key inside `docs` rejected; non-boolean `inject` rejected. Regen class: EXPECTED CHANGE scoped to the `schemas_lock` crate itself (the new field, parser branch, serializer line) with ZERO byte change to every OTHER module's generated artifacts. This is the highest-cascade edit in the feature because `schemas_lock` is a load-bearing leaf many shell paths read; pin the serialized field order and omission rules in prose, read the schema crate's patch line by line, and diff every other artifact to zero before proceeding.
- `napl init` writing `"docs": {"inject": true}` is a prompt edit to the `cmd_init` shell module plus regen (regen class: expected change scoped to the init-written lock bytes); gated by an updated `01-init-fresh` golden (sanctioned edit: the freshly-written `lock.json` gains the `docs` block). This is the one sanctioned golden edit in Slice C.

### State and schema migrations

- `lock.json` gains an optional `docs` object. Backward-compat: absent means false, so every existing tree is unaffected; a v1 binary reading a lock with `docs` present rejects it under strict parse (documented pre-1.0). Refusal: none new in this slice.

### Gates

The gate battery, with the emphasis that the FULL pre-existing corpus stays byte-identical except the single sanctioned `01-init-fresh` edit. The `schemas_lock` regen is expected-change scoped to its own crate; every other module's generated artifact diffs to zero.

### Rollback story

If the `schemas_lock` regen changes artifact bytes, revert the prompt edit and restore the locked crate from git; the field addition is the only change and must be provably inert. The `cmd_init` prompt edit is revertible module by module without touching pure crates.

### Estimated gen cycles

3. One for `schemas_lock` (possibly two if the serializer reorders keys on first regen), one for the `cmd_init` shell regen.

---

## Slice D: injection-ON (design Part 2). Split into D1 through D4.

Slice D is the feature's core and does not fit one gen cycle. It is split so each sub-slice reaches a green corpus before the next. D1 builds the pure emission-and-strip modules and their property tests with no pipeline wiring. D2 wires the gen pipeline including the durable recovery marker. D3 adds reconcile, healing, blame labeling, and display metadata. D4 adds activation migration and the lexical ban validator with its escape contract. Every sub-slice keeps `docs.inject` reachable only through fixtures that opt in.

### D1: pure emission, strip, inject, item grammar, comment content

#### Goal and scope

Implements the pure heart of design Part 2's emission contract: the two injected shapes (module header with the fixed sentinel first line, and item doc runs), the structural strip as a total pure function, the inject as its inverse under the projection inputs, the supported item grammar v1 (Rust file-scope `pub fn|struct|enum|const|trait|type`; TS top-level `export function|const|class|interface|type`), and the comment-content assembly (sentences whose attribution file-ranges intersect an item span, verbatim, prompt order, deduplicated, whitespace-normalized; module header from the opening paragraphs before the first heading). No pipeline, no validator, no CLI wiring in D1.

#### New and changed conformance scenarios

D1 asserts through the equivalence corpus primarily; conformance scenarios for the round-trip land here as pure-through-CLI checks only where a debug surface exists. The property scenarios are:

- Equivalence property tests (authored first) for `docs_strip` and `docs_inject`: `strip(inject(x)) == x` for all doc-comment-free `x` (the only inputs that exist post-validation), and `inject(strip(y)) == y` for `y` produced by `inject` under the same projection inputs. Domains narrowed to the enforced contract per design r3 finding 7.
- `docs_item_grammar` equivalence cases: each supported Rust and TS item kind detected with its span by brace or semicolon matching; detection-failure on a pathological file returns "no items" cleanly (never panics, never a build failure).
- `docs_comment_content` equivalence cases: sentence selection by range intersection, prompt-order dedup, whitespace normalization, module-header assembly from opening paragraphs, and the frontmatter-tests-as-Examples DROP (no test-to-item provenance in v1).

#### New and changed .napl prompts

- `selfhost/core/docs_item_grammar.napl` (new). `deps: []`. Prose: the exact supported item grammar per target, span-by-brace-or-semicolon, and the explicit non-support list (`pub(crate)`, impl members, macros, re-exports, default exports, overloads). Detection failure returns an empty item set for that file; it can never fail a build.
- `selfhost/core/docs_strip.napl` (new). `deps: [docs_item_grammar]`. Prose: strip removes the file-head header block whose first line matches the sentinel text, and every contiguous `///` run (TS `/** */` block) immediately above a recognized public item; non-doc comments untouched; total pure function of file text, no log, no positions, no hashes.
- `selfhost/core/docs_inject.napl` (new). `deps: [docs_item_grammar]`. Prose: inject emits the module header (first line the fixed sentinel `Generated by NAPL from <prompt-file> — the prose below is the source specification.`) and item-doc runs immediately above each recognized item with no blank line between block and item. Note: the sentinel text in the design contains an em dash; it is a fixed pinned string quoted verbatim from the design and reproduced exactly in the prompt and goldens (this plan's own prose contains no em dashes; the pinned artifact string is data, not plan prose).
- `selfhost/core/docs_comment_content.napl` (new). `deps: [schemas_attribution, ast_parse]`. Prose: content assembly from attribution ranges and AST sentences as specified above.

#### State and schema migrations

None in D1. These are pure modules with no persisted output of their own; the display-metadata record type was already added to `schemas_attribution` in Slice A and stays unpopulated until D2.

#### Gates

The gate battery, with the property-test equivalence corpus for strip and inject green and the item-grammar and content corpora green. No conformance golden changes in D1 (no pipeline wiring yet).

#### Rollback story

Four independent leaf-ish modules; a non-converging one is parked on the escape-hatch list without blocking the others (strip and inject depend on item-grammar, so item-grammar converges first). No shell change to revert.

#### Estimated gen cycles

5. One per module, plus a second attempt for item-grammar's span-matching edge cases.

### D2: gen pipeline: entry strip, views, validate-inject, durable commit marker

#### Goal and scope

Implements design Part 2's gen pipeline (r2 criticals 1-2 and r3 findings 3-6) and the durable recovery marker (r3 finding 5, r4-hardened). This is predominantly prompt work on the `cmd_gen` shell module with two new schema modules. It wires: entry strip scoped to the unlock set with in-memory strip VIEWS for locked files; the `force x full` invariant; the validate-then-inject step producing documented files staged to temp paths; the commit with backup, `.napl/gen-commit.json` marker, fsync, fixed install order, lock 0444, then marker deletion; recovery under a held gen.lock (roll forward or back, idempotent); and journal patches rebuilt documented-to-documented so replay reproduces documented bytes.

#### New and changed conformance scenarios

- `101-inject-header-and-item-docs`: a `docs.inject: true` module; the generated file gains the module header (sentinel first line) and item-doc runs above each supported item. Asserts the documented file bytes, the journal patch in documented space, the attribution v2 display-metadata per-file record, AND the `injectedGen` scalar set to the run's journal gen number (D2 is where the scalar is first written; this scenario is its write test).
- `102-inject-double-gen-noop` (multi-phase via the runner extension): two consecutive gens with unchanged prompt and unchanged agent output produce identical documented bytes and NO second patch. Asserts no-op discipline through the injection path.
- `103-inject-forced-incremental-multifile`: the pinned `force x full` corner case: a forced incremental regen of a multi-file module with mixed locked and unlocked files; strip scope equals the unlock set, locked files read through views, documented bytes on disk unchanged for locked files. Asserts per-file records and that the locked files' disk bytes stay documented.
- `104-gen-commit-recovery-forward`: a marker present at startup with all staged files existing and hash-matching; recovery under gen.lock rolls FORWARD and installs. Fixture seeds `.napl/gen-commit.json` plus staged files; asserts final installed tree and marker deleted.
- `105-gen-commit-recovery-back` (multi-phase via the runner extension): a marker present with a staged file missing or hash-mismatched; recovery rolls BACK via backups and truncates the journal to the recorded byte length; phase two runs recovery again and asserts an identical tree (idempotence on bytes, not on absence of a crash). Asserts the restored tree and journal length in both phases.
- `106-status-interrupted`: `napl status` with a marker present reports `interrupted` exit 1 and NEVER recovers (recovery only happens under a held gen). Asserts the interrupted report and exit code.

#### New and changed .napl prompts

- `selfhost/schemas/schemas_gen_commit.napl` (new). `deps: [schemas_ordered_map]` if ordering is needed, else `deps: []`. Prose: the marker document shape `{module, target, staged->final list, expected final hashes, backup list, pre-commit journal byte length}`, its strict parser, and the forward-versus-back decision predicate (forward iff every staged file exists and hash-matches). Pure schema and predicate; the fsync, install order, and lock steps are shell.
- The pipeline orchestration (entry strip in place after unlock, in-memory views for locked files, validate-inject step ordering, commit sequence, recovery under gen.lock, stale-lock takeover before marker handling, journal documented-space patches) is prompt work on the `cmd_gen` skeleton shell module plus `selfhost/cli/state_io.napl`, gated by scenarios `101` through `106`. This is the single heaviest prompt edit in the feature: the `cmd_gen` prompt gains the marker protocol as behavioral contract (write-before-install, fsync, fixed install order, lock 0444, delete-after-lock, recovery predicate under held gen.lock only). Regen class: expected change for both (`cmd_gen` scoped to the pipeline stages named above; `state_io` scoped to marker read/write and backup paths); any patch outside those scopes halts. Budget extra regen attempts here and read every patch line.
- `selfhost/cli/driftdetect_replay.napl` (verify no change needed). Design states `driftdetect_replay` is unchanged because patches are documented-to-documented; confirm by re-running its equivalence and the replay scenario. Expected-no-op risk: NONE if truly untouched; do not edit it.

#### State and schema migrations

- New durable marker `.napl/gen-commit.json`, written before install and deleted after lock, interpreted only under a held gen.lock. `napl status` reports its presence as `interrupted` and refuses to recover. Attribution v2 documents now carry populated per-file display-metadata records (written fresh every gen; staleness or corruption of these degrades displays only, never correctness) AND the load-bearing `injectedGen` scalar, written on every successful injected gen as part of the attribution write in the commit sequence, asserted by scenario `101`.
- Journal: patches and `hash_after` computed from the previous documented baseline to the staged documented output; map hashes likewise documented-space. Replay reproduces documented bytes exactly.

#### Gates

The gate battery, plus explicit recovery-idempotence check (run `105`'s recovery twice, converge) and the double-gen no-op (`102`) proving injection is deterministic. Confirm `driftdetect_replay` equivalence unchanged.

#### Rollback story

The marker protocol is the highest-risk shell in the feature (a bug can corrupt state). Mitigation is that scenarios `104` through `106` exercise forward, back, and interrupted paths on synthetic trees before any real gen runs the new commit path; the new commit path ships behind `docs.inject`, so a bug cannot corrupt a `docs`-off tree. If a recovery bug appears, revert the `cmd_gen` prompt edit, restore the prior locked crate from git, and re-derive the prose (surgical journal handling per the incremental-regression precedent, never wholesale checkout of shared state); the `schemas_gen_commit` crate is inert without the shell that writes markers.

#### Estimated gen cycles

6. One for `schemas_gen_commit` plus a likely second for its forward-or-back predicate edge cases; two to three for the `cmd_gen` pipeline prompt (the feature's hardest regen, budget failures); one for `state_io`. This sub-slice also consumes the most patch-review time.

### D3: reconcile with labeled intent, healing metadata rewrite, blame labeling, display range-shift

#### Goal and scope

Implements design Part 2's reconcile (r3 critical 2), healing and moved-files metadata rewrite (r2 finding 6, r3 finding 10), blame labeling (r2 finding 10), the drift semantics simplification (r2 critical 1: one drift class, no reconstruction in status), and the display range-shift transform (r2 finding 9).

#### New and changed conformance scenarios

- `107-inject-tampered-comment-drift`: editing a generated doc comment in a locked file is DRIFT, one class, ordinary report; the unified diff shows only comment lines changed. Asserts the standard drift report and exit code (no docs-versus-code classification).
- `108-inject-reconcile-comment-edit` (multi-phase via the runner extension): a user edits a doc comment; reconcile's task carries BOTH the original recorded drift diff (labeled "edits to toolchain-generated doc comments: fold the INTENT into the prompt prose") AND the stripped current code as the code baseline; the agent amends the prompt; the module goes stale; the next gen restores projection coherence. Fixture extends `72-reconcile-drift` with a doc-comment edit; asserts the two labeled layers appear in the agent input.
- `109-inject-item-detection-degrade`: a file where item detection fails; item docs are skipped, the module header is still injected, a mapl `note` is recorded (kind `note`, promptLines the first body line, appended by the toolchain independent of LLM mapl), and gen proceeds. Asserts header-only output and the note.
- `110-inject-incremental-after-injection`: an incremental gen following a prior injected gen; strip-view isolation preserved, only the selected subset stripped in place. Asserts unchanged locked documented bytes.
- `111-inject-healed-move-metadata`: a healed file move rewrites map paths AND the `path` fields of attribution-v2 display metadata in the same run, as SEQUENTIAL writes; a partial update degrades labels until the next gen. Asserts the moved paths in both places after a clean heal.
- `112-inject-multifile-per-file-records`: a multi-file module produces one display-metadata record PER FILE. Asserts the per-file record set.
- `113-inject-crlf-file`: a CRLF target file injects and strips correctly with `\r` preserved. Asserts documented bytes with CRLF intact.

#### New and changed .napl prompts

- `selfhost/cli/reconcile_derive.napl` (extend). Add the labeled-two-layer task construction: the original recorded drift diff verbatim with the doc-comment-intent prefix, plus the stripped current code as the code baseline. Behavior prose specifies both labels exactly. Regen class: EXPECTED CHANGE scoped to an additive branch keyed on the presence of doc-comment lines; the existing derivation for non-doc-comment drift must stay byte-identical (asserted by the existing reconcile scenarios). Extend `tests:` with a doc-comment-edit case before genning.
- Display range-shift transform: a small pure helper, either folded into `docs_inject` or a new `selfhost/core/docs_range_shift.napl` (new, `deps: []`). Prose: for insertion of `n` lines above documented line `N`, range start at least `N` gets `+n`, range end at least `N` gets `+n` (endpoints transform independently, per block in ascending anchor order); ranges never include injected lines. Prefer the dedicated module to keep `docs_inject` a pure emitter.
- Healing metadata rewrite and blame label layer are prompt edits to the `healing_io` shell module (which must exist by D3, see Prerequisites) and the `cmd_blame` shell prompt (regen class: expected change; `healing_io` scoped to the sequential metadata-path rewrite, `cmd_blame` scoped to the additive label layer), gated by `111` and by extending blame scenarios. Blame history stays journal-derived on documented bytes; the metadata adds an optional label layer only; `blame --gen` output is journal-derived and unaffected by metadata. Confirm existing blame scenarios stay byte-identical except any sanctioned label additions.
- Drift simplification: the design introduces no docs-versus-code drift class, so this is a verification item, not an edit: confirm `selfhost/cli/statusclass_io.napl` and the `cmd_status` shell prompt classify a doc-comment edit as ordinary DRIFT with no special casing. Gated by `107` and by confirming existing drift scenarios stay byte-identical. If any classification special-casing crept in, remove it via prompt edit plus expected-patch regen.

#### State and schema migrations

- Display metadata is written fresh every gen and rewritten (best-effort, sequential) by healing; its staleness degrades labels only. No new persisted schema beyond what Slice A and D2 added. Reconcile records the accepted baseline as today and leaves the module stale.

#### Gates

The gate battery. Explicitly confirm: existing reconcile scenarios (`72`, `73`) stay byte-identical for non-doc-comment drift; existing drift scenarios stay byte-identical under the single-class simplification; existing blame scenarios stay byte-identical except sanctioned label additions.

#### Rollback story

`reconcile_derive` is a shared module; if its regen changes non-doc behavior, revert and re-derive with a tighter additive branch. `docs_range_shift` is a pure leaf, parkable. The healing, blame, and statusclass prompt edits are independently revertible module by module (prompt revert plus locked-crate restore from git).

#### Estimated gen cycles

4. One for `docs_range_shift`, one for the `reconcile_derive` extension (with no-op verification on the non-doc path), one each for the `healing_io` and `cmd_blame` shell edits.

### D4: activation migration and the lexical ban validator with escape contract

#### Goal and scope

Implements design Part 2's activation migration (r4 critical 1), the lexical ban validator (r4 highs), and the escape contract for the unsupported constructs. The ban validator makes the strip unambiguous by construction: with `docs.inject` on, agent output must contain no doc-form comments; the injection step validates this before injecting and fails the attempt with a pinned error fed back to the agent, sharing the existing MAX_ATTEMPTS budget.

#### New and changed conformance scenarios

- `114-inject-ban-correction-on-retry`: agent output contains a `///` doc comment on attempt 1; the validate step fails the attempt with the pinned error appended to the retry task; attempt 2 (from the rejected workspace) is clean and succeeds. Asserts the pinned error text in the attempt-2 agent input and the successful outcome.
- `115-inject-ban-exhaustion`: agent output keeps emitting doc-form comments across all attempts; gen fails with files left unlocked, sharing the MAX_ATTEMPTS budget with test-gate and attribution failures. Asserts the exhaustion failure and unlocked files.
- `116-activation-refuse-on-preexisting-docs`: flipping `docs.inject` false to true on a tree whose locked files already contain doc-form blocks makes the first gen REFUSE with the migration message listing the blocks and the fold-or-delete instruction. Asserts the refusal message and exit code, and that nothing is stripped.
- `117-activation-clean-noop`: activation on a clean tree (no pre-existing doc comments, the common case) is a no-op; gen proceeds normally. Asserts normal gen with no migration message.
- `117b-activation-second-gen` (multi-phase via the runner extension): after a successful injected gen, a SECOND gen of the same module must NOT refuse, even though its locked files now contain tool-emitted doc blocks. This pins the transition signal below.
- `117c-activation-metadata-deleted` (multi-phase): after a successful injected gen, DELETE the display-metadata records from the attribution document (keep `injectedGen`); the next gen proceeds normally and rewrites them. Pins that display metadata is not load-bearing for activation.

The transition signal (closing a design rev 5 underspecification, to be folded back into the design as a rev 6 note): the attribution v2 document, which is ALREADY load-bearing and protected by the attribution gate, gains one scalar field `injectedGen` (the journal gen number of the last injected gen for that (module, target); absent means never injected). The signal is deliberately NOT derived from display-metadata records, which the design pins as never correctness-affecting; deleting or corrupting display metadata must not change gen behavior, and scenario `117c` pins exactly that (seed a post-injection tree, delete the display records, second gen proceeds normally). The activation scan applies per module: a doc-form block in a locked file of a module whose attribution document lacks `injectedGen` is pre-existing and refuses; with `injectedGen` present the blocks are tool-emitted by construction (the ban plus injection pipeline is the sole writer, and content tampering is caught as ordinary map-hash drift independently of this signal). A missing or corrupt attribution document is already a hard failure of the existing attribution machinery, no new fragility class. A forged `injectedGen` on a tree with pre-existing docs would bypass the migration refusal, and because the scalar is optional state with no protected-hash relationship, the forgery itself is NOT detectable; state forgery is outside the toolchain's guarantees entirely (prevention is never offered, and detection applies only where protected hashes or state relationships diverge). The consequence of this particular forgery is bounded: pre-existing doc blocks get structurally stripped on the next gen instead of refusing, which is the pre-rev-5 behavior, losing hand-written docs but corrupting nothing. `injectedGen` is added to the schema in Slice A (optional, absent default, expected-change regen scoped to the schema crate) and first written in D2. Interrupted activation needs no special case: refusal happens before any write.

#### New and changed .napl prompts

- `selfhost/core/docs_ban_validator.napl` (new). `deps: []`. Prose defines "doc position" lexically per target over a comment-aware scan that EXCLUDES string, char, and raw-string literals: Rust any `///` or `//!` line outside literals; TS any `/** */` block whose next non-trivia token opens an exported declaration, plus file-head `/** */`. States the pinned rejection error text. Declares the excluded-but-unsupported constructs (Rust doctests, `# Safety` sections, TS JSDoc type directives) as UNSUPPORTED under injection in v1: projects needing them keep `docs.inject` off. `tests:` cover: a bare `///`, a `//!`, a `/** */` above an export, a file-head `/** */`, a `///`-looking sequence inside a string literal (NOT flagged), and a `# Safety`-style doc that is nonetheless a `///` line (flagged, because it is doc position; support is future work).
- Activation scan and refusal is a prompt edit to the `cmd_gen` shell module (it reads locked files on the first gen after activation, runs `docs_ban_validator` over them, and refuses with the block listing), gated by `116` and `117`.
- Validate-reject retry integration (sharing MAX_ATTEMPTS, re-running from the rejected workspace with the pinned error appended) is a prompt edit to the gen attempt loop in the same `cmd_gen` shell module, gated by `114` and `115`. Sequence D4's `cmd_gen` edit after D2's has landed and no-op-verified, so the two edits to the feature's hottest prompt never interleave.

#### State and schema migrations

- No new persisted schema. Activation is a first-gen-after-flip behavior gate: refuse on pre-existing doc-form blocks, no-op on a clean tree. Refusal behavior is the migration path (fold content into the prompt or delete, regen with docs off, then activate).

#### Gates

The gate battery. Confirm the ban validator's lexical scan does not false-positive on doc-comment-looking content inside string literals (the highest-value corpus case) and does flag genuine doc positions.

#### Rollback story

`docs_ban_validator` is a pure leaf; a non-converging validator is the feature's second-highest risk (it could reject all attempts). Mitigation: the equivalence corpus pins accept-versus-reject cases first, and the validator ships behind `docs.inject`, so a false-positive-prone validator cannot block a `docs`-off tree. If the validator over-rejects, revert to a narrower lexical scan and widen against the corpus. Activation refusal shell is revertible in isolation.

#### Estimated gen cycles

4. One for `docs_ban_validator` with a likely second for the literal-exclusion edge cases; one to two for the `cmd_gen` activation-and-retry prompt edit (sequenced strictly after D2's `cmd_gen` edit has no-op-verified).

---

## Slice E: `napl docs` (design Part 3)

### Goal and scope

Implements design Part 3: `napl docs [--out docs-dist] [--allow-dirty] [--json-only]`, the versioned byte-pinned `docs.json` (`docsVersion: 1`), rendering from locked baselines and state only, HTML-escaped output, module-to-filename slug with collision as hard error, CSP meta with zero scripts in v1, atomic replace via temp dir and swap, and exit codes 0/1/2. Refuses a non-empty out dir without the `napl-docs` marker; refuses dirty without `--allow-dirty`.

### New and changed conformance scenarios

- `118-docs-golden-json`: `napl docs --json-only` on a small multi-module tree; asserts the full `docs.json` byte-for-byte, keys in schema-declared order, arrays ordered by (module name, target, file path, line), POSIX-normalized project-relative paths, LF only, no trailing whitespace, minimal escapes.
- `119-docs-dirty-refusal`: `napl docs` on a tree with a dirty module refuses (exit 1) without `--allow-dirty`.
- `120-docs-allow-dirty`: `--allow-dirty` renders the dirty module's LOCKED baseline with the dirty flag set (the flag says disk differs; the page shows last-good truth). Asserts the dirty flag in `docs.json`.
- `121-docs-out-dir-safety`: refuses a non-empty out dir lacking the `napl-docs` marker with exit 2 (pinned here as a USAGE error per the design's 0/1/2 split: the user pointed the tool at a directory that is not a docs output dir; nothing is dirty or corrupt about the project. Fold this pin back into the design as a rev 6 note). Accepts and atomically replaces when the marker is present.
- `122-docs-slug-collision`: two modules slugging to the same filename is a HARD error. Asserts the error and exit code.
- HTML-escape assertion: folded into `118` by including a module whose prose contains characters requiring escaping; asserts the escaped bytes in `docs.json`.
- `123-docs-html-contract`: a full (non `--json-only`) run; asserts the emitted HTML files themselves: byte-golden expected HTML for the small fixture tree, which pins by construction the CSP meta tag present, zero `script` elements, all inserted text HTML-escaped (the fixture prose includes `<`, `&`, and a quote), the slugged filenames, and the `napl-docs` marker file. `docs.json` alone cannot prove the renderer emitted no raw text or scripts; this scenario proves it on bytes.

### New and changed .napl prompts

- `selfhost/schemas/schemas_docs.napl` (new). `deps: [schemas_ordered_map]`. Prose: the `docs.json` document shape with `docsVersion: 1`, its strict parser, and the canonical serialization discipline (object keys in schema-declared order, arrays ordered by the pinned tuple, POSIX-normalized project-relative paths, LF only, no trailing whitespace, minimal escapes), matching the existing pinned-bytes JSON writer family used by `schemas_map`. `tests:` include a round-trip byte-identity case and an ordering case. This is the module the design calls out as itself a prompted module with byte-pinned given/expect tests.
- `selfhost/core/docs_render.napl` (new, if a pure render-model assembly is separable). `deps: [schemas_docs, schemas_attribution, ast_parse]`. Prose: assemble the render model (module pages, dep-graph index, attribution annotations, machine margins, generation history) from locked baselines and state; HTML-escape all text; slug modules to `[a-z0-9-]` with collision detection. Honest scope: documents prompts plus state, not a target-language symbol index.
- `napl docs` subcommand: new shell prompt `selfhost/cli/cmd_docs.napl` with `deps: [docs_render, schemas_docs, state_io, fsutil_io]` (final set per actual composition) plus a dispatch line in the `main` glue prompt; reads locked baselines and state, calls `docs_render` and `schemas_docs` serialization, writes the out dir atomically via temp-and-swap, enforces the marker and dirty rules, sets exit codes. Gated by `118` through `123`, the HTML byte golden included.

### State and schema migrations

- `docs.json` is an OUTPUT artifact, not persisted toolchain state; `docsVersion: 1` is pinned. No `.napl` state schema changes. The out dir carries a `napl-docs` marker so the tool refuses to clobber unrelated directories.

### Gates

The gate battery, with `docs.json` byte-identity and the `123` HTML byte golden as the central assertions and the serializer's own given/expect equivalence green first. Slice E is not done while any of `118` through `123` fails.

### Rollback story

`schemas_docs` and `docs_render` are additive pure modules with no dependents; parkable on the escape-hatch list. `cmd_docs` is an additive shell module; deleting its prompt and crate removes the subcommand without touching any other slice.

### Estimated gen cycles

6. One for `schemas_docs` (byte-pinned serializer, likely one clean pass given the `schemas_map` precedent), two for `docs_render` (render-model breadth and slug/escape edges), one to two for the new `cmd_docs` shell module, one for the `main` glue dispatch regen.

Total across the plan: roughly 43 gen cycles (A 8 with the two reader regens, B 7 with the wasm binding, C 3, D1 5 with item-grammar retry, D2 6, D3 4, D4 4, E 6), of which about half are expected-change shell regens with enumerated scope. The runner extension and the TS reader/helper work are not gen cycles and add real time on top. At the campaign's observed cadence this is comparable to two shell-wave batches. Treat all per-slice numbers as provisional until Checkpoint 0 resolves the landed shell-prompt inventory.

---

## Dependency graph between slices

- A (attribution v2) is the root prerequisite: D1's `docs_comment_content` and D2's display-metadata population both read the v2 document, and E's `docs_render` reads attribution annotations.
- B (AST) is a prerequisite for D1 (`docs_comment_content` consumes AST sentences) and E (`docs_render` consumes the AST).
- C (flag inertness) depends only on the lock-schema change; it can land any time after A but before any D sub-slice turns the flag on in a fixture.
- D1 depends on A and B. D2 depends on D1 (it wires the pure modules) and on A's display-metadata record type. D3 depends on D2 (reconcile, healing, blame operate on the injected pipeline's output). D4 depends on D1 (the ban validator and strip share the emission contract) and on D2 (the validate step lives in the pipeline); D4 can proceed in parallel with D3 because the ban validator and activation are independent of reconcile and healing.
- E depends on A, B, and D2 (it renders injected, documented baselines and attribution display metadata); it does not depend on D3 or D4.

Critical path: A -> B -> D1 -> D2, then D3, D4, and E all start; the feature is done when all three finish. E does NOT wait for D3 or D4. The runner extension and Checkpoint 0 precede Slice A.

## What can parallelize

- A and B share no code and can be built concurrently by two agents (A touches `schemas_attribution` and the path-layout shell prompts; B adds the independent `ast_parse` leaf and the `cmd_parse` shell module). The one shared surface is the `main` glue prompt if B lands its dispatch line while A is regenerating shells; serialize glue regens.
- Within D1 the four pure modules are near-independent: `docs_item_grammar` first, then `docs_strip`, `docs_inject`, and `docs_comment_content` in parallel (the first two depend on item-grammar; content depends on AST and attribution, not on strip or inject).
- D3 and D4 parallelize after D2 (reconcile and healing versus ban validator and activation touch disjoint shell and disjoint pure modules).
- C can be slotted opportunistically after A, in parallel with B or D1, since its only pure change is the lock schema.
- All parallel work obeys the AGENTS.md rule: agents never stage or commit; the orchestrator verifies and commits with explicit-path staging; no broad `git add` with concurrent agents.

## Go/no-go checkpoints (human review required)

1. After A: v2 layout migrates the existing 45 modules with zero artifact-byte change on regen. No-go if any artifact hash moves.
2. After B: `ast_parse` equivalence green across spans, CRLF, BOM, problems, and literal fallbacks; `napl parse --json` goldens byte-identical. No-go if span offsets are inconsistent between raw-byte and line coordinates.
3. After C: the FULL pre-existing corpus stays byte-identical except the single sanctioned `01-init-fresh` edit. No-go if any artifact changes, because it means the lock-schema regen gained meaning.
4. After D1: strip and inject property tests green on the narrowed domains. No-go if `strip(inject(x)) == x` fails for any doc-comment-free input.
5. After D2: recovery forward, back, and interrupted scenarios green and recovery idempotent; double-gen no-op holds. This is the highest-stakes checkpoint; no-go on any marker-protocol nondeterminism.
6. After D3 and D4: existing reconcile, drift, and blame scenarios byte-identical for non-doc paths; ban validator does not false-positive on string-literal content; activation refuses on pre-existing docs and no-ops on clean trees. No-go if the ban validator over-rejects clean output.
7. After E: `docs.json` byte-identical, the HTML byte golden (`123`) green with CSP meta, zero scripts, and escaped text pinned on bytes, and the out-dir marker safety holds. No-go if the serializer is not byte-stable or the HTML contract fails.
8. Standing: every regen carries its declared class (expected no-op with zero patches, or expected change with enumerated scope); any patch outside its declaration halts for investigation.

## Risks and mitigations

- Prompt edits cascading regens. The highest-cascade edits are `schemas_lock` (Slice C) and `schemas_attribution` (Slice A), both load-bearing leaves read by many shell paths. Mitigation: additive-optional-field-only edits with serialized key order and default-omission pinned in prose; both are expected-change regens scoped to their own crate, so diff every OTHER module's artifact to zero and treat any patch outside the enumerated scope as a halt-and-investigate. Extend `tests:` before genning so the corpus catches meaning drift.
- The ban validator rejecting all attempts. If `docs_ban_validator` over-rejects (for example flagging doc-comment-looking content inside string literals), every `docs.inject` gen exhausts its attempts. Mitigation: pin accept-versus-reject cases in the equivalence corpus FIRST, including the string-literal-exclusion case; ship the validator behind `docs.inject` so a false-positive-prone validator never blocks a docs-off tree; the task text states the ban up front so genuine rejections are rare; on over-rejection, narrow the lexical scan and widen against the corpus.
- Recovery marker protocol bugs corrupting state. A bug in the D2 commit or recovery sequence can leave a tree half-installed or a journal truncated wrong. Mitigation: scenarios `104` through `106` exercise forward, back, and interrupted on synthetic trees before any real gen uses the new path; recovery runs only under a held gen.lock and `napl status` never recovers (reports `interrupted`, exit 1); recovery is idempotent (a recovery after a recovery crash converges); the new commit path ships behind `docs.inject`, so it cannot corrupt a docs-off tree; stale gen.lock takeover follows existing stale-lock rules before any marker handling.
- Attribution v2 migration breaking the existing 45 modules. A path or precedence bug could orphan or clobber real attribution state. Mitigation: scenarios `90` through `93` cover v1-read, migrate-on-gen, multi-target non-clobber, and orphan-note before any real tree migrates; readers accept both layouts during transition; migrate-on-gen deletes a v1 file only when its `target` equals the generated target, leaving other targets' v1 data until their own gen migrates it; the go/no-go checkpoint after A requires zero artifact-byte change across representative multi-target regens.
- Item-grammar false negatives. Detection failure degrades to fewer comments, surfaced as a mapl note; it can never fail a build (scenario `109`). Accepted as a degrade, not a risk to correctness.
- Display-metadata staleness. Degrades labels only, never correctness; displays fall back to journal ownership. Accepted per design.

## Out of scope (explicit)

The following are declared unsupported in v1 by the design and are NOT built by this plan:

- Rust doctests under injection. Projects needing them keep `docs.inject` off. A validated non-strippable representation is future work.
- Rust `# Safety` (and similar) doc sections under injection. Same disposition.
- TS JSDoc type directives under injection. Same disposition.
- Grammar constructs the AST does not recognize in v1: block quotes, HTML, links, images, reference definitions, footnotes, tables, indented code, setext headings. These parse as literal paragraph text; recognizing any of them is a future `astVersion` bump, not this plan.
- Item kinds outside the v1 grammar: Rust `pub(crate)`, impl members, macros, re-exports; TS default exports and overloads.
- Frontmatter-tests-as-Examples: dropped from v1 (no test-to-item provenance relation exists).
- A docs-versus-code drift classification. Deleted by design; one drift class only, the unified diff shows which lines changed.
- A target-language symbol index in `napl docs`. The docs surface documents prompts plus state, not target symbols.
- Any new gate. The design introduces none; this plan introduces none. The conformance corpus remains the sole gate for CLI behavior.
- A versioned machine-marker comment syntax (the sentinel-collision escalation path). Accepted as convention for v1; built only if real-world collisions appear.
