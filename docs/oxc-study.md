# oxc Study — Reuse Map for the NAPL Rust Rewrite

Deep read of `oxc-reference` (oxc-project/oxc, a 40+ crate Rust JS-toolchain monorepo)
to extract exactly what the NAPL Rust port (`napl-core` / `napl-agent` / `napl-cli` /
`napl-lsp`) should adopt, imitate, or ignore. Every claim cites a real file (paths relative to
`oxc-reference/`; `rust/` = NAPL's Rust workspace). Hand this to every Rust-phase agent.

---

## 1. Key findings (read this first)

1. **oxc_diagnostics is a thin wrapper over `miette`** — `crates/oxc_diagnostics/Cargo.toml`
   depends on `miette` + `cow-utils` + `percent-encoding`, nothing else. `OxcDiagnostic`
   (`src/lib.rs`) is a hand-rolled builder that `impl miette::Diagnostic`. **Do not depend on
   oxc_diagnostics; depend on `miette` directly and copy the builder pattern.** miette's
   `GraphicalReportHandler` gives the red-squiggle/underline pretty CLI rendering NAPL wants.
2. **The LSP is not a separate binary.** It is `oxlint --lsp` — `apps/oxlint/src/main.rs`
   lazily builds a Tokio runtime only when `command.lsp` is set, otherwise runs a synchronous
   Rayon CLI. NAPL should do the same: one `napl` binary, `napl lsp` subcommand spins the async
   server; everything else stays sync.
3. **`oxc_language_server` is a generic, tool-agnostic LSP scaffold.** It knows nothing about
   linting. The actual tool is injected as `Arc<dyn ToolBuilder>` producing `Box<dyn Tool>`
   (`crates/oxc_language_server/src/tool.rs`). The **same `oxc_linter` crate** powers both the
   `oxlint` CLI and the LSP `Tool` impl — zero logic duplication. This is precisely the
   `napl-core` ↔ (`napl-cli` + `napl-lsp`) boundary. See §5.
4. **LSP framework = `tower-lsp-server` v0.23** (a maintained fork of the now-stale `tower-lsp`),
   with `features=["proposed"]` for pull-diagnostics (`crates/oxc_language_server/Cargo.toml`).
   Use it, not raw `lsp-server`.
5. **Distribution = `@napi-rs/cli`, not cargo-dist.** oxc builds one standalone Rust binary per
   target with `cross`/`cargo-zigbuild`, then `napi create-npm-dirs` / `napi artifacts` /
   `napi pre-publish` generate per-platform npm packages and wire `optionalDependencies` on the
   shim package (`.github/workflows/release_apps.yml`). Version sync is a `package.json`-driven
   trigger. Full recipe in §4.
6. **UTF-8 byte offset → LSP UTF-16 position is a 30-line naive linear scan**
   (`crates/oxc_language_server/src/position.rs`) — no `ropey`, no line-index cache. Copy it
   almost verbatim; it is exactly what NAPL needs for prose spans and it already handles
   multi-byte + emoji correctly (has tests for 🍄).
7. **oxc does no server-side debouncing and no server-side file watching.** Debounce is the
   editor's job (config `run: onType|onSave`); file watching is delegated to the client via
   `client/registerCapability` + `FileSystemWatcher` glob patterns (`worker.rs::init_watchers`,
   `backend.rs::did_change_watched_files`). NAPL's expensive gen-on-save **must** add its own
   debounce — this is a divergence, see §5 and §7.
8. **oxc's CLI arg parser is `bpaf` (linter) / `pico-args`, not `clap`.** NAPL's plan names
   clap; that is fine (clap is heavier but ergonomic) — just know oxc is not evidence *for* clap.
9. **`Span` is `{ start: u32, end: u32 }` byte offsets** (`crates/oxc_span/src/span.rs`), u32 to
   halve size. NAPL's span scanner should match: u32 byte offsets, convert to line/col only at
   the LSP/render boundary. `Span` impls `Index<Span> for str` so `&source[span]` just works.
10. **Arena allocation (`oxc_allocator`) is a parser hot-path optimization — irrelevant to NAPL.**
    NAPL processes small prose files and shells out to an LLM; allocator arenas, `oxc_ast`
    bump-allocated trees, and codegen machinery are all pure overhead. Skip.
11. **Testing spine = `insta` snapshots + external conformance suites.** `insta` (workspace dep,
    v1.48) is the one testing crate worth adopting now. Conformance-suite-as-submodule scale
    (Test262/Babel) is oxc-specific; NAPL's analog is golden `.napl`→attribution fixtures.
12. **Workspace hygiene is copy-pasteable gold** (§2): `[workspace.lints]` clippy config,
    `.clippy.toml` `std::HashMap`→`FxHashMap` ban, release profile (`lto="fat"`, `panic="abort"`,
    `codegen-units=1`, `strip`), per-package opt-level overrides, and `just ready` +
    `cargo-shear` + `typos-cli` as the "don't break main" gate.
13. **`similar` (v3.1.1) does unified diff + patch**, and **`schemars`** emits JSON Schema from Rust
    types (oxc ships a generated `configuration_schema.json`). The former may retire NAPL's
    hand-rolled LCS in `rust/crates/napl-core/src/text_diff.rs`; the latter can generate a `.napl`
    frontmatter schema for editor validation. See §8.

---

## 2. Workspace mechanics — adopt mostly verbatim

Root `Cargo.toml` (oxc) vs `rust/Cargo.toml` (NAPL, currently minimal):

| Item | oxc (`Cargo.toml`) | NAPL action |
| --- | --- | --- |
| `[workspace.package]` shared meta | edition 2024, `rust-version` MSRV N-2 policy | Adopt the *pattern* (shared version/edition/license/repo). NAPL pins edition 2021 + rust 1.80 — fine; bump to 2024 when ready. |
| `[workspace.lints.clippy]` | huge curated `pedantic`+`nursery`+`cargo` block with reasoned `allow`s (lines 30-120) | **Adopt.** NAPL already has a small version; expand using oxc's as the menu. Keep `pedantic=warn`. |
| `.clippy.toml` | disallows `std::HashMap/HashSet` → `FxHashMap`; disallows allocating `str::to_lowercase` etc. → `cow_utils` | Adopt the HashMap ban (pull `rustc-hash`). The `cow_utils` bans are perf-paranoia; **skip** — NAPL isn't allocation-bound. |
| `.rustfmt.toml` | `style_edition=2024`, `use_small_heuristics="Max"`, `use_field_init_shorthand` | Adopt verbatim. |
| `[profile.release]` | `opt-level=3, lto="fat", codegen-units=1, strip="symbols", panic="abort"` | **Adopt verbatim** for the shipping binary. |
| `[profile.dev]` / `[profile.test]` | `debug=false` (faster builds); `insta.opt-level=3`, macros `opt-level=1` | Adopt; add `insta.opt-level=3` once insta is in. |
| `rust-toolchain.toml` | pinned `channel="1.97.1"`, `profile="default"` | NAPL pins `stable` — pin an exact version instead for reproducible CI. Add `components=["rustfmt","clippy"]`. |
| `Cross.toml` | 5 lines — env passthrough for `cross` cross-compilation | Adopt when you add the release matrix (§4). |
| `deny.toml` | `cargo-deny` license+advisory+ban policy (11 KB) | Adopt a trimmed version (license allowlist + RUSTSEC advisories). Runs in `.github/workflows/deny.yml`. |
| `justfile` | task runner: `ready`, `fmt`, `test`, `lint`, `doc`, `coverage`, `watch` | Adopt a small `justfile`: `ready = fmt + check + test + lint`. See `justfile` lines 40-95. |

`just ready` (lines 40-55) is the "don't break main" gate: `git diff --exit-code`, `typos`,
`cargo fmt`, `cargo check`, `test`, `clippy -D warnings`, `cargo doc -D warnings`, `git status`.
Mirror this as NAPL's pre-push / CI check.

---

## 3. Crate-by-crate verdicts

### oxc_diagnostics → **use `miette` directly, imitate the builder**
`src/lib.rs`: `OxcDiagnostic` carries `{ message, labels: Labels, help, note, severity, code,
url }` and builds fluently (`.error(msg).with_label(span.into()).with_help(..).with_error_code(scope,num)`).
`Labels` come from `miette::LabeledSpan`; a label is `(offset,len)` + text. Rendering:
`DiagnosticService` (`src/service.rs`) is an mpsc channel + pluggable `DiagnosticReporter` trait
(`src/reporter.rs`) → `miette::GraphicalReportHandler` prints the underlined, colored report.
`reporter.rs::Info` shows how to turn a labeled span back into 1-based line/column for
machine-readable output (the LSP/JSON path).
**NAPL:** define `NaplDiagnostic` with the same shape, `impl miette::Diagnostic`, reuse
`GraphicalReportHandler` for `napl status`/`gen` CLI errors and the ambiguity/assumption squiggles.
Map `.mapl` entry kinds → severity (ambiguity=Error, assumption/no-op=Warning, note=Advice).

### oxc_span → **imitate `Span`, copy the LSP conversion**
`Span { start: u32, end: u32 }` byte offsets, `Index<Span> for str`, `.label(text)` helper making
a `LabeledSpan`. NAPL's `span scanner` (phase-1 goal) should produce these. The only nontrivial
part — byte-offset → `(line, utf16-char)` — is already solved in
`oxc_language_server/src/position.rs::offset_to_position` (naive scan, treats only `\r \n \r\n` as
breaks per LSP, correct for 2/3/4-byte chars). **Copy that function.** Skip oxc_span's `edit_distance`,
estree serialize, `source_type` — JS-specific.

### oxc_allocator → **skip.** Arena/bump allocator for AST nodes. NAPL has no AST and no hot path.

### oxc_parser + oxc_ast → **defer; feasible later as an optional `napl-verify` dep**
Embedding these to statically inspect *generated TypeScript* (e.g. confirm an attribution line
range really covers a `function`/export) is feasible: `oxc_parser::Parser::new(&allocator, src,
source_type).parse()` yields a `Program` with `Span`s on every node; walk it with `oxc_ast_visit`
to collect real function/decl spans, then intersect with attribution ranges. Cost: pulls
`oxc_allocator` + `oxc_ast` + `oxc_parser` (heavy, but published on crates.io at a stable version).
**Verdict: not phase 1.** It only helps the TypeScript target and only as a *validation* nicety;
keep attribution model-driven for now, revisit once TS pipeline is solid. If adopted, gate behind a
cargo feature so non-TS builds stay lean.

### oxc_linter → **imitate the CLI↔LSP single-source pattern, not the rule engine**
The valuable idea is architectural: one crate computes diagnostics; both the CLI (`apps/oxlint`)
and the LSP (`Tool` impl) call it. Rules register into a generated enum and each produces
`OxcDiagnostic`s with spans that flow unchanged to CLI rendering and LSP `publishDiagnostics`.
NAPL's analog: `napl-core` computes `status`/`gen`/attribution/`.mapl` entries once; `napl-cli`
renders them via miette, `napl-lsp` maps them to LSP `Diagnostic`. The `.mapl` entry `kind`s are
NAPL's "rules". Skip oxc's visitor/rule-codegen infrastructure — massively over-scoped for NAPL.

---

## 4. Distribution — exact npm recipe (answers the standing open question)

oxc ships each app (oxlint, oxfmt) as: **(a)** a standalone Rust binary per platform (GitHub
release + used by editor extensions) **and (b)** an npm shim package with per-platform
`optionalDependencies`. NAPL only needs the npm path plus a curl-installable binary; here is the
mechanism, generalized to `napl`:

**Package layout** (`npm/oxlint/`):
- `npm/oxlint/package.json` — the shim. Declares `"bin": { "oxlint": "bin/oxlint" }`, `"files":
  ["dist","bin/oxlint",...]`, and a `"napi": { "binaryName", "packageName": "@oxlint/binding",
  "targets": [ ...19 rust triples... ] }` block that drives the tooling.
- `npm/oxlint/bin/oxlint` — 3-line Node shim: `#!/usr/bin/env node` + `import "../dist/cli.js";`.
  (oxc's `dist/cli.js` is generated and loads a NAPI `.node` addon because oxlint embeds JS
  plugins. **NAPL does not need NAPI** — NAPL's binary is pure Rust and just spawns `claude`. So
  NAPL's shim should instead resolve the platform package's *binary* and `execFileSync` it — the
  classic pattern. Keep the `napi`-cli plumbing for generating the platform packages + optionalDeps,
  but ship a real binary, not a `.node`.)

**CI build+publish** (`.github/workflows/release_apps.yml`):
1. **Trigger**: `push` to `main` touching `npm/oxlint/package.json` (version bump *is* the release
   trigger). `.github/actions/check-version` compares the file's version against
   `unpkg.com/oxlint@latest/package.json`; `version_changed` gates all downstream jobs.
2. **Build matrix** (`build-oxlint`): ~20 targets. Native binary via
   `cross build --release -p oxlint --target=<triple>`; musl/riscv via `cargo-zigbuild`; FreeBSD
   via a `cross-platform-actions` VM. Each job uploads a zipped Rust binary artifact
   (`.github/actions/archive-binary`) for the GitHub release.
3. **Publish** (`publish-oxlint`, `environment: release`, `id-token: write` for provenance):
   - `pnpm napi create-npm-dirs` → scaffolds one dir per target under `npm/oxlint-release/`.
   - `pnpm napi artifacts` → drops each built binary into its platform dir.
   - `pnpm napi pre-publish -t npm` → **publishes the sub-packages and injects
     `optionalDependencies` into the main package** (this is the key step: the shim ends up
     depending on `@oxlint/binding-darwin-arm64` etc., each with its own `os`/`cpu` fields so npm
     installs only the matching one).
   - `publish-if-needed.sh` publishes the shim with `--provenance --access public`.
4. **GitHub release** (`github-release`): `cargo release-oxc changelog` builds notes; binaries
   uploaded with the `rust-` prefix stripped, so users can `curl` a stable asset URL — this is the
   curl-installer backbone.
5. **Version sync**: `prepare_release_apps.yml` runs weekly (cron), `cargo release-oxc update`
   bumps Cargo crate versions + npm `package.json` + changelog in one PR. Merging that PR flips the
   version and step 1 fires. **Cargo and npm versions are bumped together by one tool.**
6. **Smoke test** (`smoke`): after publish, `npx oxlint@<version> test.js` on win/mac/linux/alpine.

**NAPL replication checklist**: (i) `napl/` shim package with `bin/napl` that resolves+execs the
platform binary via `optionalDependencies`; (ii) `@napl/binding-<triple>` platform packages with
`os`/`cpu`; (iii) reuse `@napi-rs/cli`'s `create-npm-dirs`/`artifacts`/`pre-publish` to generate
them; (iv) `cross` + `cargo-zigbuild` build matrix (start with mac-arm64, mac-x64, linux-x64-gnu,
linux-x64-musl, win-x64 — 5 targets, add later); (v) version-bump-triggers-release; (vi) publish
GitHub release assets at predictable URLs for a `curl | sh` installer.

---

## 5. `napl-lsp` blueprint — dedicated deep dive of `oxc_language_server`

This crate is the direct template. Scaffold `napl-lsp` from this section.

**Framework & why.** `tower-lsp-server` v0.23 with `features=["proposed"]`
(`crates/oxc_language_server/Cargo.toml`). It is the maintained fork of `tower-lsp` (async,
`#[tower_lsp_server::async_trait] impl LanguageServer`), chosen over raw `lsp-server` because it
gives the full async trait, JSON-RPC plumbing, and client-request helpers
(`client.publish_diagnostics`, `client.register_capability`, `client.configuration`) for free.
`"proposed"` unlocks pull-diagnostics types.

**Entry point & packaging.** `lib.rs::run_server(name, version, worker_manager)` wires
`LspService::build(|client| Backend::new(...)).finish()` over `tokio::io::stdin/stdout` and
`Server::new(stdin, stdout, socket).serve(service).await`. There is **no dedicated binary** — the
`oxlint` binary calls it: `apps/oxlint/src/main.rs` builds a Tokio `Runtime` only when `--lsp` is
passed (comment there: keep the CLI path sync/Rayon so `#[tokio::main]` doesn't spawn idle worker
threads per lint). Editors launch `oxlint --lsp` over stdio. **NAPL:** `napl lsp` subcommand → build
Tokio runtime → `napl_lsp::run_server("napl", version, WorkerManager::new(Arc::new(NaplToolBuilder)))`.

**The crate boundary (no duplication).** `oxc_language_server` is 100% generic. It defines two
traits in `src/tool.rs`:
- `ToolBuilder: Send+Sync` — `build_boxed(root_uri, options) -> Box<dyn Tool>`, plus
  `server_capabilities(...)` to advertise features and a `shutdown` hook.
- `Tool: Send+Sync` — the work surface: `run_diagnostic`, `run_diagnostic_on_save`,
  `run_diagnostic_on_change` (all `-> Result<Vec<(Uri, Vec<Diagnostic>)>, String>`), `run_format`,
  `get_code_actions_or_commands`, `execute_command`, `get_watcher_patterns`,
  `handle_watched_file_change`, `handle_configuration_change`, `remove_uri_cache`. All default to
  no-op, so a tool implements only what it supports.

`apps/oxlint/src/lsp/server_linter.rs` provides `ServerLinterBuilder: ToolBuilder` wrapping the
**same `oxc_linter` crate the CLI uses** — the linter logic lives in one place. **NAPL mirror:**
`napl-core` exposes `status`/`gen`/`attribution`/`blame`; `napl-lsp` ships a `NaplTool: Tool` that
calls `napl-core` to produce diagnostics (ambiguity/assumption squiggles from `.mapl`), hover
(attribution + blame lines), and go-to-def; `napl-cli` calls the identical `napl-core` fns.

**Backend structure** (`src/backend.rs`). `Backend` holds `client: Client`, `server_info`,
`worker_manager: WorkerManager`, `capabilities: OnceCell<Capabilities>`, and
`file_system: Arc<LSPFileSystem>` (in-memory map of open docs). It `impl LanguageServer` with the
full lifecycle: `initialize` (reads workspace folders / falls back to deprecated `root_uri` /
single-file mode; negotiates push-vs-pull diagnostics from client capabilities), `initialized`
(requests `workspace/configuration`, registers file watchers), `did_open`/`did_change`/`did_save`/
`did_close`, `did_change_configuration`, `did_change_watched_files`, `did_change_workspace_folders`,
`code_action`, `execute_command`, `diagnostic` (pull), `formatting`, `shutdown`.

**Workers & threading.** A `WorkspaceWorker` (`src/worker.rs`) owns one root URI + one
`RwLock<Option<Box<dyn Tool>>>` + its options; `WorkerManager` (`src/worker_manager.rs`) routes a
file URI to its most-specific worker and supports dynamic single-file workers. Concurrency model:
Tokio multi-thread runtime; per-worker `tokio::sync::RwLock`/`Mutex`; open-file map is a lock-free
`papaya::HashMap` (`type ConcurrentHashMap`). `Tool::run_diagnostic` is **synchronous** and runs
inline inside the async handler (oxlint is fast). **NAPL divergence:** `napl gen` spawns `claude`
and takes seconds — do **not** run it inline. `NaplTool` should return quickly and drive gen on a
`tokio::task` with progress via `client.show_message`/a status notification; guard with a cancel
token so a newer edit supersedes an in-flight gen.

**Document sync, debounce, watched files.** Text sync is `FULL` (whole document each change;
`did_change` stores `content_changes[0].text` into `file_system`). **No server-side debounce** —
behavior is controlled by client option `run: "onType" | "onSave"` (see README table); onType lints
in `did_change`, onSave in `did_save`. File watching is **delegated to the editor**:
`worker.init_watchers()` returns `FileSystemWatcher` glob patterns (`get_watcher_patterns`) that
`initialized` registers via `client.register_capability`; the editor then sends
`workspace/didChangeWatchedFiles`, handled in `backend.rs::did_change_watched_files` (re-lint / ask
client to refresh). **NAPL:** map gen-on-save to `run:onSave` + **add a real debounce** (e.g.
`tokio::time::sleep` reset per keystroke, or only act on `did_save`) because gen is costly; register
watchers for `**/*.napl` and `.napl/**` so external prompt edits re-trigger.

**Diagnostics modes.** Push (`textDocument/publishDiagnostics`) vs Pull
(`textDocument/diagnostic`, requires client `refresh` support) — negotiated from
`ClientCapabilities` in `Capabilities::from` (`src/capabilities.rs`). Prefer pull when available.
NAPL can start push-only for simplicity.

**Position conversion.** `src/position.rs::offset_to_position` — copy it. Byte offset (from
`napl-core` spans) → LSP `Position{line, utf16 char}`.

**VS Code client.** The `editors/` dir is absent from this shallow clone, but the server contract
is fully specified in `crates/oxc_language_server/README.md`: client sends `initializationOptions`
as `[{ workspaceUri, options }]`, supports `workspace/configuration`, registers watchers on
request. NAPL's existing `apps/vscode` extension already bundles a TS server; the Rust `napl lsp`
becomes a drop-in stdio replacement — the extension just spawns `napl lsp` instead of node.

---

## 6. Testing patterns

- **`insta` snapshots** (workspace dep v1.48; `cargo insta review`). Used across
  isolated-declarations, semantic, coverage. **Adopt now** for `napl-core`: snapshot attribution
  YAML, blame output, `.mapl` rendering, diff patches. `[profile.dev.package] insta.opt-level=3`.
- **`Tester` inline pass/fail helper** (linter): each rule file has `Tester::new(...).test_and_snapshot()`.
  NAPL analog: table-driven prompt→diagnostic fixtures.
- **Conformance suites as git submodules** (Test262/Babel/TypeScript/Prettier, thousands of cases,
  `just conformance`). This is oxc's scale answer to 500 contributors. **NAPL analog, scaled down:**
  a `fixtures/` set of real `.napl` projects with committed expected attribution/journal, run in CI
  — NAPL's `rust/fixtures/` already exists for this. Snapshots track *failures*, not passes.
- **`similar-asserts`** for readable diff assertions; **`cargo-shear`** (unused deps) + **`typos-cli`**
  in `just ready`. Cheap, adopt.

---

## 7. Explicit anti-matches (do NOT copy)

| oxc thing | Why NAPL must skip it |
| --- | --- |
| `oxc_allocator` arena / bump allocation | Perf trick for million-node ASTs. NAPL handles small prose files + shells to an LLM; arenas add unsafe complexity for zero benefit. |
| `oxc_ast` / `oxc_codegen` / traverse / visitor codegen | NAPL's IR is contract-level YAML, explicitly **not** an AST (DESIGN.md §Layer 2). Building or generating syntax trees is the LCD trap NAPL rejects. |
| NAPI (`.node`) embedding for the CLI | oxlint embeds JS plugins so it must run inside Node. NAPL's binary is standalone Rust that spawns `claude`; ship a real binary + thin JS shim, skip napi-derive. |
| `.clippy.toml` `cow_utils`/`str::to_lowercase` bans | Allocation paranoia justified only on a hot lexer path. Ignore; keep the `FxHashMap` ban. |
| 20-target release matrix, OpenHarmony/riscv/s390x, weekly cron release, Discord webhooks, ecosystem-CI, `monitor-oxc` | Monorepo-scale machinery for a 40-crate project. NAPL starts with ~5 targets and a manual/tag-triggered release. |
| Running diagnostics **synchronously inline** in the LSP handler | Fine for a fast linter, wrong for NAPL where gen spawns an agent. NAPL must go async + debounce + cancel (see §5). |
| `saphyr`/`oxc-yaml-parser` (comment-preserving YAML AST) | Over-engineered for NAPL frontmatter. Use `serde_yaml` (NAPL already does) for schema-validated parse; add `schemars` only to *emit* a schema. |
| `bpaf`/`pico-args` for CLI | Not a mistake, just not aligned — NAPL chose `clap`; keep clap. |

---

## 8. Prioritized external-crate shopping list (with oxc evidence)

Order = adopt-now → later. Versions are oxc's current picks (`Cargo.toml`), safe starting points.

| Crate | Version | Use in NAPL | oxc evidence |
| --- | --- | --- | --- |
| `miette` (`features=["fancy"]`) | 3.x | Diagnostics with spans + pretty CLI rendering (ambiguity squiggles, `napl status` errors). Depend directly. | `oxc_diagnostics` is a thin wrapper over it; `GraphicalReportHandler`, `LabeledSpan`. |
| `thiserror` | 2 | Typed error enums in `napl-core`. | Already in `rust/Cargo.toml`; oxc-wide pattern. |
| `serde` / `serde_json` / `serde_yaml` | 1 / 1 / 0.9 | Schemas: `.napl` frontmatter, IR YAML, `map.json`, journal JSONL. | Workspace deps; NAPL already has them. |
| `insta` | 1.48 | Snapshot tests for attribution/blame/mapl/diff. | Workspace dep; `cargo insta` across crates. |
| `rustc-hash` (`FxHashMap`) | 2 | Default map type; ban `std::HashMap`. | `.clippy.toml` disallowed-types. |
| `tower-lsp-server` (`features=["proposed"]`) | 0.23 | `napl-lsp` framework. | `oxc_language_server/Cargo.toml`. |
| `tokio` (`rt-multi-thread, io-std, macros, process, time`) | 1.52 | LSP async runtime; `tokio::process` to spawn `claude`; `time` for debounce. | LSP crate; main.rs lazy runtime. |
| `similar` | 3.1 | Unified diff + patch for journal `patch` fields, prompt diffs, reconcile deltas — evaluate vs NAPL's hand-rolled `text_diff.rs`. | Workspace dep (`similar`, `similar-asserts`). |
| `similar-asserts` | 2 | Readable test diffs. | Workspace dep. |
| `sha2` + `hex` | 0.10 / 0.4 | Content hashing (staleness/drift, journal `hashBefore/After`). | NAPL already has; oxc uses `hmac-sha1` variants. |
| `ignore` + `walkdir` | 0.4 / 2.5 | Gitignore-aware discovery of `.napl` files / target src trees. | Workspace deps (linter file walking). |
| `papaya` | 0.2 | Lock-free concurrent map for the LSP open-file store (later). | `ConcurrentHashMap` in `oxc_language_server`. |
| `schemars` | 0.9 | Emit JSON Schema for `.napl` frontmatter (editor validation, shipped in npm pkg). | oxc emits `configuration_schema.json`. |
| `tracing` + `tracing-subscriber` | 0.1 / 0.3 | Structured logging shared by CLI + LSP. | LSP + `init_tracing()` in oxlint main. |
| `notify` (NOT from oxc) | latest | `napl gen --watch` CLI file watching. | oxc has none server-side; uses external `watchexec`. Pick `notify` for in-process CLI watch. |
| `@napi-rs/cli` (npm, dev) | latest | Generate platform packages + optionalDeps for distribution. | `napi create-npm-dirs/artifacts/pre-publish` in release workflow. |
| `cross` + `cargo-zigbuild` (tools) | — | Cross-compile the release matrix (musl via zig). | `release_apps.yml` build steps. |

Tooling (not crates): `cargo-deny` (`deny.toml`), `cargo-shear`, `typos-cli`, `just`, `insta` CLI.

---

## 9. Surprises that change the NAPL Rust plan

1. **The LSP is a subcommand, not a crate-binary.** Reconsider whether `napl-lsp` needs to be a
   separate published binary at all — make it a library crate that `napl-cli` invokes via `napl lsp`,
   exactly as oxlint does. Fewer artifacts to distribute; the VS Code extension spawns `napl lsp`.
2. **You get the CLI↔LSP "single source of truth" for free via a `Tool` trait.** Design `napl-core`'s
   public API as the thing both surfaces call, and put a thin `Tool`-style adapter in `napl-lsp`.
   Bake this boundary in from day one — retrofitting is painful.
3. **Distribution needs no NAPI and no cargo-dist.** The whole thing is: standalone Rust binary +
   `@napi-rs/cli` generating optionalDependency platform packages + a version-bump-triggers-CI flow.
   This closes the "npm distribution" open question — copy oxc's `release_apps.yml` shape at 5 targets.
4. **Server-side debounce/cancel is NAPL-net-new.** oxc gives no template because its work is cheap.
   Budget real design for: debounce gen-on-save, cancel superseded gens, stream progress to the
   editor. This is the single biggest LSP divergence.
5. **`similar` may retire NAPL's hand-rolled LCS.** Before hardening `napl-core/src/text_diff.rs`,
   prototype the journal-patch + reconcile-delta paths on `similar` — it already produces
   git-style unified diffs and can apply them.
6. **UTF-16 position handling is already solved and tested (incl. emoji).** Given NAPL's emoji
   filenames and prose spans, `position.rs` is a gift — copy it and its test suite verbatim.
