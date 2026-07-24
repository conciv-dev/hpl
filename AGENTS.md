# AGENTS.md

Operational rules for agents and contributors in this repo. The product story lives in README.md; this file is the non-negotiables that keep work consistent. Every rule here exists because its violation was caught in review — treat them as failed tests, not suggestions.

## Source of truth

- The conformance corpus (`conformance/`, byte-identical goldens) is the SOLE spec of CLI behavior. Green corpus is the gate for toolchain changes; golden edits must be explicitly sanctioned, never incidental.
- Runner: `cd conformance && node --experimental-strip-types runner/cli.ts`. Never the pnpm wrapper.
- `selfhost/*.napl` prompts are source; `selfhost/.napl/` is toolchain-owned state (generated crates are locked 0444 — never hand-edit).

## Code style (all languages)

- Zero code comments. Write self-explanatory code. (Rust keeps existing `///` doc-comment style where the crate already uses it.)
- Functions, not classes. No IIFEs.
- TypeScript only for JS surfaces: no authored `.js`/`.mjs` files. Node scripts are `.ts` (native type stripping). Exceptions: generated artifacts (wasm pkg) and the published npm shim (`npm/napl-lang/bin`, CommonJS by design).
- Filenames: kebab-case everywhere. Component names stay PascalCase; files do not.
- Formatting/linting: oxfmt + oxlint at the root (`pnpm format`, `pnpm lint`) — no semicolons, single quotes, printWidth 120. Run both before finishing.
- Strict TS: no `any`, no `as` escapes, named exports.

## UI work (apps/site, packages/napl-editor)

- Assemble from packages: fumadocs-ui, radix-ui, react-resizable-panels, @uiw codemirror themes, lucide. Never hand-roll UI primitives. `~/Public/web/aidx/apps/site` is the design reference of record.
- No raw `<pre>` — every code surface goes through a highlighted component (CodeMirror or the shiki CodeBlock pipeline).
- Editors: dark theme (self-contained palette — never rely on bare `--fd-*` vars, they don't exist), Geist Mono via `--napl-mono`, bounded heights with internal `.cm-scroller` scroll. Editors never grow to content height; pages never scroll because of an editor.
- SSR fallbacks must be pixel-equivalent to the hydrated component and generated from the same fixture data — one source of truth, zero layout shift, identical text.
- User-facing copy: no "tab/pane/panel", no slop, destination-named actions ("Go to src/greeting.ts:1"), real-editor language. If a label wouldn't appear in VS Code or a well-edited book, rewrite it.
- Verification is visual: playwright e2e (aidx vitest.e2e pattern) plus screenshots at 1440×900 and 1280×720 in BOTH site themes, and you must actually look at every screenshot. curl/status-codes are banned as UI evidence. jsdom green means nothing.
- Never touch the dev server on :4600 — run your own vite on a free port.

## Toolchain / selfhost work

- `napl gen` runs strictly foreground (Bash timeout 600000), one module at a time. Never background a gen; never end a turn to "wait".
- Prompt edits: `deps:` frontmatter is the single dependency declaration (gen-time enforced). Where deps exist, the body keeps the imperative "use its public API — do not reimplement its types or logic". Dep-enumeration prose is redundant; behavioral imperatives are load-bearing spec — know the difference before trimming.
- Expected-no-op discipline: a regen after a prompt edit should produce 0 file patches with an honest mapl entry. A regen that produces real patches means the prompt lost or gained meaning — stop and investigate, do not plow on.
- Escape-hatch list (docs/selfhost-map.md) for modules that won't converge in 3 attempts — with the reason. Never force convergence.
- Gate battery after toolchain/selfhost changes: conformance byte-identical, `cargo test --workspace`, `cargo clippy --workspace --all-targets` clean, `selfhost/equivalence` tests, generated-tree `cargo test`, `napl status` fully clean.

## Process

- Agents never commit or stage — the orchestrator verifies and commits with explicit-path staging. Broad `git add` with concurrent agents has burned us; don't.
- Keep a running handoff file in the session scratchpad updated after every completed item — assume you can be terminated at any moment and a successor continues from that file alone.
- pnpm repair when the workspace breaks mid-run: `corepack pnpm install --no-frozen-lockfile --config.confirmModulesPurge=false` at the root.
- If a decision isn't yours to make (scope change, golden edit, convention change), stop and report — don't decide by momentum.
