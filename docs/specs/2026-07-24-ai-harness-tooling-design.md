# AI harness tooling: full powers for coding agents over NAPL

Date: 2026-07-24. Status: future plan, not scheduled, revised after one adversarial review round (gpt-5.6-sol: 5 high, 5 medium, verdict sound with fixes; all ten findings addressed in this revision). Foundations are independent of the AST/docs feature; the tools that consume its surfaces are sequenced against its slices (see Phasing).

## Problem

NAPL is a language whose compiler back end is a coding agent, and whose own toolchain is written by coding agents. Yet a harness working ON a NAPL project today gets none of the language's intelligence. It edits `.napl` files as plain text, shells out to `napl gen` and scrapes stdout, and never sees what the LSP serves a human in an editor: ambiguity diagnostics anchored to prompt lines, assumption warnings, hover context from the `.mapl` reply, prompt-to-code navigation through attribution, blame history from the journal. Each harness reinvents the workflow from scratch, and each reinvention risks violating the discipline the toolchain exists to enforce (editing locked generated files, skipping gates, wholesale-checkout of shared state).

For most languages, agent ergonomics are an ecosystem afterthought. For NAPL they are core infrastructure: the agent IS the compiler back end and the primary author. A harness with full powers is not a convenience layer; it is the language working as designed.

## Architecture

Three candidate shapes were considered and rejected: per-harness plugins implementing the workflow directly (N harnesses times M tools, drift where discipline matters most); an MCP server with nothing beneath it (untestable through the conformance corpus, unusable from scripts); and an MCP server layered literally on top of the JSON CLI by spawning it (process overhead and a text seam in the middle of every call).

The chosen shape is a shared typed application layer with thin wire adapters:

### Layer 0: `napl-core` (exists)

Pure derivations. Unchanged by this plan.

### Layer 1: `napl-ops`, the typed application layer (new)

Today the load-bearing generation workflow (locking, drift checks, snapshots, agent execution, tests, attribution retries, state writes, failure recovery) is private orchestration inside the CLI binary, and most command handlers print human text internally and return exit codes. No adapter can reuse that. `napl-ops` extracts it: every operation (`status`, `gen`, `reconcile`, `blame`, `test`, `init`, `parse`, `docs`) becomes a typed function returning typed results, and long-running operations emit a typed event stream (`GenEvent`: attempt started, gate result, mapl entries, patch summary, final outcome). The same extraction covers the read-side queries the LSP currently derives privately in LSP-shaped code: diagnostic facts, attribution matches, navigation matches, hover facts, and project-state reads become protocol-neutral query functions. The LSP is rewritten to convert those facts to LSP wire types; the MCP server converts the same facts to MCP shapes; neither depends on the other.

Renderers over the event stream and results, all thin: the human CLI renderer (today's output, byte-identical, conformance-gated), the JSON CLI renderer, and the MCP adapter.

### Layer 2a: the JSON CLI contract

Every command accepts `--format json` and emits exactly one versioned final JSON document on stdout (`cliJsonVersion: 1`, byte-pinned serialization like `map.json`). Uniform envelope: `{cliJsonVersion, command, ok, result | error, warnings}`; errors use a structured error envelope; stderr carries logs only and is never part of the contract; exit codes keep their existing meanings; subprocess output (test runs) is captured into the result, not interleaved. `gen --format json --stream` additionally emits NDJSON `GenEvent` lines on stdout with the final envelope last (stream mode is a distinct contract; plain mode buffers). Commands covered: `status`, `gen`, `blame`, `reconcile`, `test`, `init`, `parse`, `docs`. `watch` and `lsp` are excluded (interactive), `build` is deprecated and excluded. Note `status` today also performs move healing and writes `map.json`; the ops extraction must surface that as an explicit, documented effect of the status operation, not a surprise.

This layer is the lowest common denominator (any harness that can run a subprocess gets structured results) and the layer the conformance corpus gates, which makes the shared ops layer spec-tested by construction.

### Layer 2b: `napl mcp`

An MCP server hosted by the CLI binary, stdio transport. Stdout carries JSON-RPC messages exclusively (the MCP stdio rule); all logging goes to stderr. The server calls `napl-ops` directly, the same functions the CLI renders, so behavior cannot drift between surfaces.

Long-running operations: an MCP tool call returns one result, so `gen` cannot stream NDJSON through a tool response. The `GenEvent` stream maps to `notifications/progress` when the client supplies a progress token, and gen/reconcile return an operation id immediately; `operation_status` polls and `operation_cancel` cancels. If the MCP Tasks facility stabilizes, it replaces the polling pair behind the same ops-layer stream.

### Layer 3: thin per-harness packages

Configuration and prompt guidance ONLY, never workflow logic; anything behavioral belongs in layers 1 and 2 where the corpus and equivalence tests gate it.

## Tool surface (v1 inventory)

Tools (verbs):

- `project_info`: project discovery from a directory (locate `.napl/`, validate lock and map, report modules, targets, declared deps as a graph). The entry point every session starts with.
- `init`: create a project (same semantics as `napl init`).
- `prompt_read` / `prompt_write` / `prompt_patch`: read, replace, or CAS-edit a `.napl` file. Writes validate frontmatter against the schema and run the AST parser, returning structured problems instead of a failed gen later. `prompt_patch` takes a base content hash and fails on mismatch (safe concurrent editing). Writes are confined to toolchain-discovered prompt files under the project root: no caller-supplied arbitrary path, symlinks resolved and revalidated immediately before atomic replace, refusal on `.napl/` state paths and locked artifacts.
- `prompt_validate`: the same validation with no write.
- `gen`: run generation for a module through the ops layer, full gate battery, progress via the operation mechanism above.
- `status`: full project status as structured data (per-module state, drift, staleness, interrupted marker), including the healing side effect explicitly flagged.
- `drift_explain`: for a drifted module, the unified diff plus the sanctioned next actions as data.
- `reconcile`: run reconcile, returning the derived task and outcome; `dry_run: true` returns the plan without acting.
- `docs_build`: run `napl docs` (once it exists) and return the output summary.
- `journal_query`: generation history with cursor-based paging (by module, gen number, file), structured patch access.
- `attribution_lookup`: bidirectional, prompt lines to generated code lines and reverse, from the attribution documents.
- `mapl_query`: the machine's reply for a module (ambiguities, assumptions, no-op explanations, notes) with prompt-line anchors. Parity honesty: `.mapl` entries carry line ranges, so this is LINE-RANGE parity with the editor experience, not word-level anchoring, unless the AST feature later persists finer spans.
- `parse`: the `astVersion: 1` AST for a prompt document.
- `recovery_inspect`: when an interrupted-gen marker exists, report the marker contents and the forward-or-back plan as data (see safety model).
- `operation_status` / `operation_cancel`: the long-running operation pair.
- `conformance_run`: dev-build-only flag, toolchain repo development.
- `lock_inspect`: which files are locked, by which module and gen. There is deliberately NO `lock_override` tool.

Resources (nouns, read-only, with MIME types and defined missing/corrupt-state errors): `napl://map`, `napl://lock`, and resource templates `napl://mapl/{module}`, `napl://attribution/{module}/{target}`, `napl://docs` when present. The journal is NOT a resource (paging belongs to `journal_query`; a resource read has no cursor).

## Safety and discipline model

Honest three-ring model, replacing any claim that the harness is bound "by construction":

1. The MCP surface is safe by construction: no tool writes generated files or `.napl/` state, `gen` and `reconcile` run every gate unconditionally because the ops-layer entry points have no bypass, both acquire the same gen.lock as the CLI, and write paths are allowlisted and revalidated.
2. The harness itself is NOT bound: it retains shell and filesystem powers and can chmod a 0444 artifact, edit state files, or run `napl gen --force` directly. No tool design changes that. The per-harness packages carry the defense: guidance steering all NAPL mutations through the MCP tools, and where the harness supports hooks (Claude Code), a warning hook on raw edits of `.napl/` state or locked paths.
3. The toolchain detects deterministically: direct edits surface as drift, state tampering surfaces as hash mismatches in `status`, and the gates refuse at the next gen. Detection, not prevention, is the recovery guarantee.

Interrupted-gen marker handling: the server does NOT refuse to start (that would withhold exactly the inspection tools a stuck harness needs). It starts in restricted recovery mode: read-only tools plus `recovery_inspect` work; mutating tools are rejected with a structured error pointing at recovery; the interrupted check reruns before EVERY mutating call, not only at startup; actual recovery runs only under a held gen.lock per the AST/docs plan's marker protocol.

## Harness matrix

- Claude Code: full MCP support, plugins with skills and hooks. The thin package is a plugin: registers `napl mcp`, ships a skill encoding the prompt-editing discipline (spec-level prose, deletable versus load-bearing sentences, expected-no-op etiquette), and a PreToolUse hook warning on raw edits of `.napl/` state or locked paths.
- Codex CLI: MCP via config (`mcp_servers` in config.toml). Package is a config snippet plus an AGENTS.md fragment with the same discipline guidance; no hook system, so ring 3 detection carries more weight.
- Antigravity: MCP-capable editor-agent surface; integration is server registration plus a rules file. Uniquely consumes both wire adapters of the same ops layer (LSP in the editor, MCP for the agent).
- Any future MCP-capable harness: one config file plus one guidance document.

## Relationship to self-hosting

Post rust-final, the toolchain law is: everything is a prompted module, pure cores and I/O shells alike (the pre-wave convention of hand-written shells, still described in `docs/selfhost-map.md`, is superseded by the campaign; a reviewer correctly noted the map still reflects the old split). Applied here:

- Pure prompted modules: the JSON envelope and event schemas (`schemas_cli_json`, byte-pinned given/expect tests), the MCP protocol framing (JSON-RPC parse/serialize, tool metadata, argument validation, dispatch selection, result encoding) as pure modules, and the ops-layer query derivations extracted from today's LSP code.
- Shell prompted modules: `cmd_mcp` (stdio loop, lifecycle, capability negotiation, cancellation wiring, gen.lock interaction) joins the cli shell layer like every other command shell, gated by scripted MCP integration scenarios (JSON-RPC on stdin, byte-pinned responses).
- Distribution glue, not prompted: the per-harness packages (plugin manifest, config snippets, rules files) and npm/vsix packaging.

The `napl-ops` extraction itself is a refactor of what will by then be prompted modules, so it lands as prompt restructuring under the expected-change regen discipline, not as hand-written code.

## Phasing

- v1 foundations (independent of the AST/docs feature): the `napl-ops` extraction with typed results and `GenEvent`, `--format json` for `status`, `gen`, `blame`, `reconcile`, `test`, `init`, the MCP server with lifecycle plus the tools that need only current state (`project_info`, `init`, prompt read/write/patch/validate minus AST problems, `gen`, `status`, `drift_explain`, `reconcile`, `journal_query`, `mapl_query`, `lock_inspect`, operations pair), and the Claude Code plugin.
- v1.5 (after AST/docs Slice A): `attribution_lookup` with dual-layout v1/v2 attribution reading, like every other reader.
- v2 (after AST/docs Slices B and E): `parse` tool and AST problems in write validation (B); `docs_build` and the `napl://docs` resource (E). Also: Codex and Antigravity packages, `recovery_inspect` against the real marker protocol (D2), dry-run everywhere.
- v3: sentence-level display joins in `attribution_lookup` (AST sentence layer); bidirectional flow: a `mapl_annotate` tool letting the harness record its own assumptions into a designated harness-notes section of the machine layer. Requires its own design pass: the mapl reply is currently toolchain-authored only, and mixing authorship needs the same rigor as the attribution model.

Out of scope at every phase: a lock-override tool, any gate-skipping gen variant, mutable access to `.napl/` state, an HTTP transport (stdio only until a real remote use case exists), per-harness workflow logic beyond configuration and guidance, and prevention-level sandboxing of the harness (ring 2 is guidance, ring 3 is detection; prevention is the harness's own permission system's job).

## Open questions

1. Does `prompt_write` return AST problems from day one (requires Slice B) or frontmatter-only validation in v1 with AST problems added in v2? Current answer: frontmatter-only in v1, staged as above.
2. Is the NDJSON stream contract versioned separately from `cliJsonVersion` or under it? Leaning under it: one version, one contract family.
3. Where do the per-harness packages live: this repo under `apps/`, or a separate distribution repo alongside the npm shim? Follows the wasm/npm distribution decision.
4. Does `status` keep its implicit healing write under the ops extraction, or does healing become an explicit operation the MCP surface exposes separately? Leaning explicit: silent state writes in a read-named tool are exactly what this plan exists to eliminate.
