# NAPL AST, docs-from-source, and doc-comment injection — design

Date: 2026-07-24. Status: rev 5 — after three adversarial review rounds (gpt-5.6-sol; r1: 14 findings; r2: 2 critical + 5 high; r3: 2 critical + 5 high with 11 of 14 r2 items resolved). Rev 3 demoted the injection log to display metadata and made strip structural. Rev 4 closes r3: doc-form comments are BANNED from agent output (validated at injection, attempt-failing) — making structural strip unambiguous by construction; strip is scoped to the unlock set with in-memory views for locked files (incremental-safe, force×full pinned); commits use a durable recovery marker; journal patches are rebuilt documented-to-documented; reconcile preserves labeled comment-edit intent alongside stripped code.

Companions: `2026-07-24-doc-site-design.md`, `2026-07-24-tutorial-local-llm-design.md`. This is a language feature. No new gate is introduced anywhere in this spec.

## Thesis (tempered)

Doc comments and doc pages are deterministic projections of (prompt text, attribution, generated code). Those inputs are hash-gated, so documentation divergence is always **detected** (as drift or staleness), never silent. Detected-never-silent is the guarantee; nothing stronger is claimed.

Deliverables: (1) napl AST parser, (2) doc-comment injection, (3) `napl docs`. User-fixed invariants: no bloat in `.napl` prompts; state is the sync backbone; out-of-sync fails gen/status.

---

## Part 0 — Prerequisite slice: target-scoped attribution v2

`.napl/attribution/<module>.yaml` holds one target's document; multi-target modules clobber each other. Fix before anything else:

- New layout `.napl/attribution/<module>/<target>.yaml`, document gains `version: 2`. v1 (old path, no version) remains readable.
- Precedence, per (module, target): if the v2 path exists it is authoritative and the v1 file is ignored for that target (even if the v1 file's `target` field matches); a v1 file whose `target` differs from the queried target is ignored as today. `napl gen` for a target writes v2 and deletes the v1 file only when the v1 file's `target` equals the generated target (other targets' v1 data stays until their own gen migrates it). Target removed from frontmatter: its attribution file becomes orphaned state — reported by `napl status` as a note, cleaned by gen.
- Readers (cli, lsp, wasm, site fixture builder) accept both layouts during transition.
- Corpus: v1-read scenario, v1→v2 migration-on-gen scenario, multi-target non-clobber scenario, orphan-note scenario. Sanctioned golden edits enumerated per scenario. Migration byte-identity claim is scoped: generated ARTIFACTS stay byte-identical; attribution state paths/contents change exactly as enumerated (r2 finding 14).

---

## Part 1 — The napl AST

### Scope discipline

A read-only view over prompt documents. It changes nothing about attribution derivation, task construction, or persisted schemas. Consumers: LSP, site/tutorial (wasm `parseDocument`), `napl docs`, injection placement. Output carries `astVersion: 1`.

### Grammar (pinned, versioned)

```
NaplDocument {astVersion, frontmatter, body: [Block]}
Block = Heading{level 1-6, atx only} | Paragraph{spans, sentences} |
        List{ordered|bullet, tight, depth ≤ 3} | CodeFence{info, text; unclosed = to EOF} |
        ThematicBreak
Span = Text | Emphasis | Strong | InlineCode
```

Not recognized in v1 — parsed as literal paragraph text: block quotes, HTML, links/images, reference definitions, footnotes, tables, indented code, setext headings. **Grammar evolution is versioned, not silent** (r2 finding 11): recognizing a new construct bumps `astVersion`; consumers pin the version they understand; documents don't change meaning under a pinned version.

### Coordinates (pinned, raw-byte)

- The parser scans the RAW document bytes itself (it does not compose on `body_lines`, which strips `\r` — r2 finding 12). Every node: `{byte_start, byte_end}` end-exclusive UTF-8 byte offsets into the full document, plus `{line_start, line_end}` 1-based inclusive (the existing repo convention).
- CRLF: `\r` belongs to its line; byte ranges include it. BOM: retained in offsets, skipped before frontmatter detection. Malformed/absent frontmatter: `frontmatter: null`, body starts at byte 0, diagnostics carried in a `problems` list (parse never fails — worst case one Paragraph).
- Offsets are u64 in Rust, JSON numbers in wasm output; documents are far below the 2^53 JS-safe bound; the wasm binding rejects inputs > 64 MiB explicitly.
- `@napl/editor` gets `byteToUtf16(text, offset)` with tests covering astral plane, CRLF, and offsets landing mid-scalar (mid-scalar input is a contract violation; helper clamps to scalar start and flags).

### Sentences: display-only query layer (r1 finding 5/6 — resolution held in r2)

Attribution stays LLM-derived line ranges, unchanged end to end. `SentenceRange` exists only for display joins; join = line-intersection with documented over-attribution. Nothing in gen/status consults sentences.

### Self-host

Parser modules are `.napl` prompts (pure, given/expect-testable). Nominated first prompt-only feature; scenarios and equivalence corpus authored before prompts.

---

## Part 2 — Doc-comment injection

### Emission format (the contract that makes strip structural)

Injected content has exactly two shapes, and the injector controls both:

- **Module header**: a contiguous run of `//!` lines (Rust) / one `/** … */` block (TS) starting at line 1 (after shebang, none expected), whose FIRST line is the fixed sentinel-by-convention text `Generated by NAPL from <prompt-file> — the prose below is the source specification.`
- **Item doc**: a contiguous run of `///` lines (Rust) / one `/** … */` block (TS) immediately above a recognized public item line, with no blank line between block and item.

**Doc comments are compiler output (the rule that makes strip unambiguous — r3 critical 1):** with `docs.inject` on, agent output MUST NOT contain doc comments (`///`, `//!`, `/** */` in doc position). The injection step VALIDATES this before injecting: any doc-form comment in agent output fails the attempt with a pinned error fed back to the agent (same retry family as the attribution and deps gates; the task text states the rule up front, so rejection is rare). This extends the project's zero-comment discipline into the artifact contract: documentation is projected from the prompt, never authored in the target. Consequence: every doc-form block in a locked generated file is tool-emitted by construction.

**Structural strip** then removes: the file-head `//!`/header block whose first line matches the sentinel text, and every contiguous `///`-run (TS: `/** */` block) immediately above a recognized public item. Total, pure function of file text — no log, no positions, no hashes. Property tests, domains narrowed to the enforced contract (r3 finding 7): `strip(inject(x)) == x` for all doc-comment-free x (the only x that exist post-validation), and `inject(strip(y)) == y` for y produced by `inject` under the same projection inputs. Non-doc comments (`//`, block comments in code position) are untouched by strip and remain governed by existing conventions. Hand-added doc comments in a LOCKED file are drift the hash gate already catches; strip never runs on tampered files.

### Supported item grammar v1 (unchanged from rev 2)

Rust: file-scope `pub fn|struct|enum|const|trait|type` in attributed `.rs` files, span by brace/semicolon matching. TS: top-level `export function|const|class|interface|type`. Detection failure on a file → item docs skipped for that file, module header still injected, machine-layer `note` recorded (schema: ordinary MaplEntry, kind `note`; promptLines = the first body line — total fallback, no heading required (r3 finding 9); the toolchain appends this note itself, independent of the best-effort LLM mapl derivation), gen proceeds. Detection can never fail a build. `pub(crate)`, impl members, macros, re-exports, default exports, overloads: not in v1.

### Comment content

Sentences whose attribution file-ranges intersect the item's span (undocumented coordinates), verbatim, prompt order, deduplicated per item; whitespace-normalized. Module header = opening paragraph(s) before the first heading. Frontmatter-tests-as-Examples: dropped from v1 (no test→item provenance relation exists).

### The gen pipeline (r2 criticals 1–2 resolution: explicit entry strip, no load-bearing log)

Per module, inside the existing attempt loop:

0. **Entry — strip scoped to the unlock set, views for the rest (r3 findings 3–4):** files the gen unlocks for the agent (full mode: all module files; incremental mode: the intersection-selected subset, selection logic unchanged) are stripped IN PLACE after unlock. Locked files keep documented bytes on disk; every derivation input that reads them (snapshot, numbered files, IR/attribution/mapl context) reads through an in-memory `strip` VIEW. One undocumented coordinate space across all derivations, zero disk mutation of locked files, incremental isolation preserved. `force × full` pinned: `--force` = drift/staleness bypass only, `--full` = mode only; strip scope always equals the unlock set + views for the rest, identically in all four combinations (corpus case: forced incremental regen, multi-file module, mixed locked/unlocked).
1. agent writes files; in-gen test gate passes (tests run on undocumented code)
2. derivations (IR, attribution, mapl) on undocumented bytes/views — pipeline unchanged from today
3. **validate + inject**: agent output must contain no doc-form comments (emission contract; violation fails the attempt with the pinned error fed back to the agent) → `inject` → documented files staged to temp paths
4. **Commit with a durable recovery marker (r3 finding 5; hardened per r4):** before install, copy each about-to-be-replaced final file into the staging dir as a backup; write `.napl/gen-commit.json` (module, target, staged→final list, expected final hashes, backup list, pre-commit journal byte length), fsync marker and staging dir; install in fixed order (sources → attribution v2 → map → journal append); fsync + lock 0444 installed files; ONLY THEN delete the marker (no unlocked-but-clean window). Recovery runs ONLY under a held gen.lock (a marker is never interpreted while another gen may own it; `napl status` never recovers — it reports `interrupted`, exit 1): roll FORWARD if every staged file exists and hash-matches, else roll BACK via backups + truncating the journal to the recorded byte length (idempotent — recovery after a recovery crash converges). Stale gen.lock takeover follows the existing stale-lock rules before any marker handling.
5. **Journal patches rebuilt, not rehashed (r3 finding 6):** patches and `hash_after` are computed from the PREVIOUS documented baseline to the staged documented output — both documented-space — so replay reproduces documented bytes exactly; `driftdetect_replay` unchanged. Map hashes likewise documented-space.

No-op discipline: unchanged prompt + unchanged agent output → identical undocumented bytes → identical injection → identical documented bytes → no patch recorded.

**Display metadata (non-load-bearing):** attribution v2 carries, per file record: `{path, undocumented_hash, documented_hash, blocks: [{anchor_line_documented, line_count, kind, item}]}` — one record PER FILE (r2 finding 3). This is written fresh every gen, used by LSP/blame/site to label comment lines and shift ranges for display. If it is stale or corrupt, displays degrade (fall back to journal ownership); correctness never depends on it. Range shifting for display (r2 finding 9): for insertion of `n` lines above documented line `N`: range start ≥ N → +n; range end ≥ N → +n (endpoints transform independently, applied per block in ascending anchor order); ranges never include injected lines.

### Status semantics (r2 critical 1 resolution: no reconstruction in status at all)

The hash gate is the whole gate. Disk hash == locked hash → clean. Mismatch → DRIFT, one class, today's report. Editing a generated doc comment is editing a locked generated file — it is drift, full stop, same as editing code. The rev-2 docs-drift/code-drift classification is DELETED (it required reconstruction under exactly the conditions where reconstruction is untrustworthy). The drift report's unified diff already shows the user that only comment lines changed — that is classification enough.

### Reconcile (r3 critical 2 resolution: intent is preserved, labeled)

Reconcile's task now carries BOTH layers, explicitly labeled: (a) the original recorded drift diff, unmodified — including any edits the user made to doc comments, prefixed in the task as "edits to toolchain-generated doc comments: these comments are projections of the prompt; fold the INTENT of these edits into the prompt prose" — and (b) the stripped current code, as the code-level baseline the prompt amendment must describe. The agent therefore sees the user's replacement prose verbatim and amends the prompt accordingly. The accepted baseline is recorded as today (on-disk edited file), the module goes stale, and the next gen (entry strip + fresh inject) restores projection coherence. Display metadata staleness degrades labels only, as below.

### Healing / moved files (r2 finding 6, r3 finding 10)

Healing rewrites map paths; it also updates the `path` fields of attribution-v2 display metadata in the same run — but these are SEQUENTIAL writes, not a transaction, and the spec does not pretend otherwise: metadata is non-load-bearing, so a partial update degrades display labels until the next gen rewrites everything. Nothing load-bearing references paths in injection metadata; the gen-commit marker protocol does not apply to healing (healing never touches locked source bytes).

### Blame (r2 finding 10)

One history: journal-patch-based on documented bytes, as today — injected lines get the generation's journal entry naturally. The display metadata adds an optional label layer ("doc comment · from <item>'s spec") on top; precedence is journal for history, metadata for labeling only; after reconcile/move the label layer degrades as above. `blame --gen` output is journal-derived and unaffected by metadata.

### Config

`lock.json`: `"docs": {"inject": true|false}`, strict-parsed; absent = false; `napl init` writes `true`. Schema change lands with the lock-schema module's prompt in the same slice.

**Activation migration (r4 critical 1):** flipping inject false→true does NOT silently strip anything. The first gen after activation scans locked files; any pre-existing doc-form block makes the gen REFUSE with a migration message listing the blocks and the instruction: fold their content into the prompt (or delete them) via a normal regen with docs off, then activate. Activation on a clean tree (the common case — our generated files have no doc comments) is a no-op.

**Validator definition + escape contract (r4 highs):** "doc position" is defined lexically per target over a comment-aware scan (string/char/raw-string literals excluded): Rust — any `///` or `//!` line outside literals; TS — any `/** */` block whose next non-trivia token opens an exported declaration, plus file-head `/** */`. Constructs the ban excludes — Rust doctests, `# Safety` sections, TS JSDoc type directives — are declared UNSUPPORTED under injection in v1: projects needing them keep `docs.inject` off (documented limitation; a validated non-strippable representation is future work). Validate-reject retries share the existing MAX_ATTEMPTS budget (a rejection consumes an attempt; the retry runs from the rejected workspace with the pinned error appended, like test-gate failures); exhaustion fails gen with files unlocked. Corpus: correction-on-retry and exhaustion cases.

---

## Part 3 — `napl docs`

Contract unchanged from rev 2 except serialization pinning (r2 finding 13):

- `napl docs [--out docs-dist] [--allow-dirty] [--json-only]`; refuses non-empty out dir without the `napl-docs` marker; atomic replace via temp dir + swap; exit 0/1/2 (success / dirty-or-corrupt / usage).
- Renders from LOCKED baselines + state, never from dirty working content; dirty modules (with `--allow-dirty`) render their locked baseline with the dirty flag set — the flag says "disk differs", the page shows last-good truth.
- `docs.json`: `docsVersion: 1`; canonical serialization = the toolchain's existing pinned-bytes JSON writer discipline (same family as map.json): object keys in schema-declared order, arrays ordered by (module name, target, file path, line), paths POSIX-normalized project-relative, LF only, no trailing whitespace, escapes minimal. The serializer is itself a prompted module with byte-pinned given/expect tests.
- Security: all text HTML-escaped (AST recognizes no HTML, so nothing is ever passed through raw); module→filename slug `[a-z0-9-]` with collision = hard error; CSP meta, zero scripts in v1.
- Honest scope: documents prompts + state (module pages, dep-graph index, attribution annotations, machine margins, generation history). Not a target-language symbol index.

---

## Part 4 — Conformance & migration slices

1. **A — attribution v2 + target scoping** (Part 0 scenarios; state-change goldens enumerated, artifact bytes unchanged).
2. **B — AST parser** + `napl parse --json` debug subcommand (fixture contract for LSP/site).
3. **C — injection flag inertness**: entire corpus with `docs` absent → generated artifacts byte-identical to pre-feature goldens (state files change only per slice A's enumeration).
4. **D — injection-ON family** (new fixtures, flag on): header + item docs; double-gen no-op byte-identity; strip/inject round-trip property scenarios; tampered-comment → DRIFT (ordinary report); item-detection-failure degrade (mapl note, header-only); incremental gen after injection; reconcile flow (comment edit → prompt amendment → regen restores); healed move with metadata rewrite; CRLF file; multi-file module with per-file records.
5. **E — `napl docs`**: golden docs.json (HTML structure asserted via docs.json only), dirty refusal, --allow-dirty flags, marker/out-dir safety, slug collision error.
6. Self-host regen of touched modules after each slice (expected no-op or sanctioned, standing discipline). Old-binary/new-state: v2 fails a v1 binary's strict parse loudly — acceptable pre-1.0, documented.

## Sequencing

Part 0 → Part 1 → Part 2 → Part 3 → site consumption. All post-shell-wave, all prompt-only authorship (scenarios first, then prompts — the standing nomination).

## Risks

- Strip mis-recognition is structurally excluded: doc-form comments cannot exist in agent output (validated, attempt-failing), so every doc-form block in a generated file is tool-emitted. Residual risk is the sentinel-collision corner (a hand-tampered header in a locked file) — already DRIFT via the hash gate; a versioned machine-marker syntax remains the escalation path if real-world collisions appear (r3 finding 8, accepted as convention for v1).
- Item-grammar false negatives degrade to fewer comments, surfaced as mapl notes.
- Display-metadata staleness degrades labels, never correctness.
- Dropping docs-vs-code drift classification loses a nicety; the drift diff shows it anyway. Revisit only with a design that needs no reconstruction-under-tamper.
