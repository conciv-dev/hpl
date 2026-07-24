# NAPL interactive tutorial + local-LLM engine — design

Date: 2026-07-24. Status: draft for review (rev 2 — detailed design).
Companion to `2026-07-24-doc-site-design.md` (site architecture, GenEngine contract). This spec covers the `/tutorial` surface, the TransformersEngine that makes it real, and the docs consolidation that shrinks around it.

## Why

The docs teach by description; a language earns belief by being felt. svelte.dev's tutorial is the reference: lesson prose on the left, live code on the right, objectives that check themselves, Solve when stuck. NAPL is unusually suited to this format — the toolchain's browser core (napl-wasm: diagnostics, attribution, mapl, replay) already runs everywhere the tutorial needs feedback, and the GenEngine seam lets the same UI run recorded replays today and live local generation the moment a model is loaded. The user requirement is explicit: the local LLM is part of the tutorial, not a later add-on — a tutorial about a generation language must generate.

## The three-track shape

- `/docs` — 9 pages, lookup material (consolidation at the end of this spec).
- `/tutorial` — the teaching track, 16 lessons, interactive.
- `/selfhost` — the proof track (exists).

---

# Part I — Tutorial surface

## Routing and content pipeline

- Routes: `/tutorial` (redirects to first incomplete lesson), `/tutorial/$lessonId` (flat ids, part encoded in the manifest — svelte.dev uses flat slugs too; parts are presentation, not URL structure).
- A lesson is a directory under `apps/site/content/tutorial/<NN>-<id>/`:
  - `manifest.yaml` — id, part, title, objective sentence, file list, check list, engine mode (`none` | `replay` | `live`), optional notes.
  - `lesson.mdx` — the prose. Compiled through the existing fumadocs-mdx pipeline (same shiki, same components available).
  - `files/` — starting prompt state (usually one `.napl`; later lessons add a second module).
  - `solution/` — the passing state. Same filenames.
  - `session.json` — recorded real generation for reference replay (present when engine mode ≠ `none`).
- Build step `scripts/build-tutorial.mjs` (sibling of `build-fixtures.mjs`, reuses its RecordedSession machinery): validates every manifest against a zod schema, checks that `solution/` actually passes its own `check:` list by running the wasm core in node, orders lessons, emits `src/lib/tutorial-index.ts` (typed) + per-lesson JSON under `src/fixtures/tutorial/`. A lesson whose solution fails its own checks fails the build — the tutorial cannot ship self-contradictory.
- All lesson pages prerender (they are static content; interactivity hydrates client-side like the playground).

## Screen anatomy

```
┌────────────┬──────────────────────┬──────────────────────────────┐
│ lesson rail│  lesson prose (MDX)  │  ObjectiveBar  ✓/✗ live      │
│ parts →    │                      ├──────────────────────────────┤
│ lessons,   │                      │  PlaygroundShell             │
│ ticks      │                      │  prompt* | generated | mapl  │
│            │                      │  [Solve] [Reset] [Generate]  │
│            │  ← prev   next →     │  engine chip (picker)        │
└────────────┴──────────────────────┴──────────────────────────────┘
```

- Rail: parts as groups, lessons with completion ticks (localStorage), free navigation.
- Prose column: 60–90 seconds of reading, one concept. MDX may embed static highlighted snippets but never a second editor.
- Right column top: ObjectiveBar — the manifest's objective sentence + one live indicator per check (label + pass/fail dot). All green → bar turns "Lesson complete", next button pulses once.
- Right column body: the existing `PlaygroundShell` (editable prompt, wasm diagnostics, generated tabs, machine tab when the lesson state has mapl content). Bounded height per the site-wide rule; internal scroll.
- Mobile (<768px): rail becomes a dropdown, prose and playground stack; playground keeps its bounded height.

## Lesson state machine

Per lesson, client-side:

```
loading → editing ──(debounced 300ms edit)→ checking → editing
   editing ──(all checks pass)→ complete (persist, stay editable)
   Solve  → load solution/ into editors → checking (passes by construction)
   Reset  → restore files/ → editing (completion tick is NOT revoked)
   Generate (mode replay) → play session.json through the replay engine
   Generate (mode live)   → engine run, below
```

- Progress store: `localStorage` key `napl-tutorial-v1` — `{completed: string[], lastLesson: string, editorState?: {[lessonId]: files}}`. Editor state persists per lesson so back-navigation keeps work; versioned key so a future curriculum change can invalidate cleanly.
- Completion is monotonic: once earned, kept (svelte.dev behavior). Reset restores files, not history.

## Objective checker

Pure function, runs in the browser, no LLM involvement:

```ts
type CheckInput = {
  files: Record<string, string>          // current editor state
  diagnostics: Diagnostic[]              // napl-wasm output for the prompt file
  frontmatter: Frontmatter | null        // napl-wasm parse
  mapl: MaplEntry[]                      // from the last completed generation (replay or live)
  lastRun: {engine: 'replay' | 'live'; completed: boolean} | null
}
type CheckResult = {id: string; label: string; pass: boolean}
```

Check kinds (v1, closed set, each a small pure function):

| kind | args | passes when |
| --- | --- | --- |
| `no-diagnostic` | `code?`, `severity?` | no matching diagnostic remains |
| `has-diagnostic` | `code`, | the diagnostic IS present (lessons that teach reading errors) |
| `frontmatter-key` | `key`, `present` \| `equals` \| `contains` | frontmatter satisfies it |
| `test-count` | `min` | frontmatter `tests:` has ≥ min entries |
| `prompt-contains` | `pattern` (regex), `target` file | body matches (used sparingly; each use needs a comment in the manifest justifying why a structural check can't express it) |
| `generation-ran` | `engine?` | a run completed this lesson (either engine unless pinned) |
| `mapl-entry` | `kind` | last run's machine layer contains an entry of that kind |
| `edited-since-solve` | — | guard so Solve alone doesn't complete "write it yourself" lessons |

The checker is unit-tested exhaustively in `@napl/editor` tests (it lives beside the playground code it reads from). Adding a check kind is a code change, not a content change — deliberate: the closed set keeps lessons honest and testable.

## Curriculum — all 16 lessons

**Part 1 — First prompt** (engine mode: none)
1. `what-is-a-prompt` — read a complete .napl; objective: none (reading lesson; complete on visit).
2. `frontmatter` — broken YAML given; fix it. Checks: `no-diagnostic(frontmatter-invalid)`.
3. `name-your-module` — missing `module:`; add it. Checks: `frontmatter-key(module, present)`, `no-diagnostic`.
4. `the-body-is-the-spec` — body says one thing, title another; make the body say what the lesson asks (write a sentence). Checks: `prompt-contains` (justified: free-writing lesson), `no-diagnostic`.

**Part 2 — Tests and generation** (engine mode: replay for 5–6, live from 7)
5. `declare-a-test` — add a given/expect to `tests:`. Checks: `test-count(min: 1)`, `no-diagnostic`.
6. `watch-it-generate` — press Generate, watch the recorded session; hover attribution. Checks: `generation-ran(replay)`.
7. `generate-for-real` — load a model (picker walkthrough in prose), run the agent step on YOUR prompt. Checks: `generation-ran(live)`. Prose sets expectations: this is the agent step; gates run in the CLI.
8. `same-prompt-different-code` — compare your model's output to the reference side-by-side; prose explains behavioral spec vs byte spec. Checks: `generation-ran`, completion-on-visit after run.

**Part 3 — The dialogue** (engine mode: replay — mapl content comes from recorded sessions)
9. `machine-says` — read a mapl file; prose maps kinds to severities. Reading lesson, complete on visit.
10. `ambiguity` — prompt with a genuinely vague sentence; recorded session flags ambiguity; rewrite the sentence until the (re-checked, wasm-side) vague pattern is gone. Checks: `mapl-entry(ambiguity)` seen, then `prompt-contains` for the disambiguated form.
11. `assumption` — make the recorded assumption explicit in the prompt. Checks: `prompt-contains` (justified), `no-diagnostic`.
12. `the-no-op-rule` — edit prose cosmetically, replay shows a no-op entry with reasoning; prose explains why silence is forbidden. Checks: `generation-ran`, `mapl-entry(no-op)`.

**Part 4 — Discipline** (engine mode: replay)
13. `drift` — generated tab is hand-editable in this lesson only; edit it, watch the drift diagnostic fire. Checks: `has-diagnostic(drift)`.
14. `reconcile` — walk the three resolutions on the drifted state (replayed reconcile session); prose explains journal provenance. Checks: `generation-ran(replay)`.
15. `deps-and-the-gate` — two modules; second declares `deps:`; prose tells the enforcement story (including the real incremental regression as a war story). Checks: `frontmatter-key(deps, contains: …)`, `no-diagnostic`.
16. `where-the-cli-begins` — closing lesson: what the browser could not show (test gate, locks, watch); install pointer; complete on visit.

Every lesson's TypeScript target keeps outputs small and cargo out of the picture. Lessons 10–12's "recorded session reacting to your edit" is scripted: the manifest pins which session state maps to which check state — the prose is written so this never claims to be live.

## Solve / Reset details

- Solve: loads `solution/` into the editor, marks `solvedUsed` in state; lessons whose pedagogy requires personal writing add `edited-since-solve` so Solve shows the answer but the visitor must retype/modify to complete.
- Reset: restores `files/`, clears the lesson's run state, keeps completion.
- Both are instant, client-side, no confirmation dialogs (state is cheap and recoverable).

---

# Part II — TransformersEngine (the local LLM)

## Contract

Implements the existing `GenEngine` interface (`run(task, files): AsyncIterable<GenEvent>`), so `PlaygroundShell`, `NaplExample`, and the tutorial consume it with zero UI forks. Emits: `task` → `file-edit` (streamed per file) → `error` on failure. It deliberately does NOT emit `attribution`, `mapl-entry`, or `lock` events — see honesty rules.

## Worker topology (aidx-proven pattern, ported)

- `@huggingface/transformers` text-generation pipeline inside a dedicated worker (`model.worker.ts` pattern from the aidx demo: `load`/`run` request messages, `progress`/`ready`/`result`/`error` responses).
- Singleton worker per site session, kept alive across route navigation — load once, live everywhere (this is what turns every docs `NaplExample` live for free).
- Device: WebGPU when `navigator.gpu` present → dtype `q4f16`; wasm fallback → `q4`. Device shown in the picker ("runs on your GPU" / "runs on CPU — slower").

## Model catalog and picker

| model | size | default when | note |
| --- | --- | --- | --- |
| SmolLM2-360M-Instruct | ~250MB | wasm-only browsers | lightest useful |
| Qwen2.5-Coder-0.5B-Instruct | ~350MB | WebGPU present | code-tuned, the intended default |
| Qwen3-0.6B | ~400MB | — | newest, picker option |
| Llama-3.2-1B-Instruct | ~800MB | — | labeled "heavy" |

- Picker lives in the playground toolbar (engine chip: "Replay" by default → "Qwen2.5-Coder · GPU" once loaded). States: `none → downloading(pct) → ready → error(retry)`.
- Metered-connection guard before any download (aidx `use-metered-connection` port): on metered connections the picker asks explicitly, showing the size.
- Choice + readiness persisted (localStorage); the browser HF cache makes reloads instant. No download ever starts without a click.

## Task construction and output contract

- Input to the model: compact system prompt = NAPL generation instructions (a distilled form of the CLI's task text: "you are the agent step; produce the target files; respect frontmatter tests") + the lesson/example's current prompt file + target (`typescript`) + expected file list.
- Context budget: total prompt capped ~3k tokens (all catalog models are 4k+); tutorial prompt files are tiny by construction; `NaplExample` files are already small. Overflow → refuse with a visible "prompt too large for the local model" state (never truncate silently).
- Output contract: fenced blocks, one per file, `path` on the fence info string. Parser is tolerant of prose between fences and of a missing final fence; it is NOT tolerant of a missing path — a file without a path is dropped and reported.
- Generation params: temperature 0.2, max_new_tokens 1024, stop on the closing sentinel. One retry on unparseable output (the retry appends the parse failure to the task — the CLI's own feedback pattern in miniature).
- Failure UX: "the model couldn't complete the agent step" panel with the raw output inspectable (honesty > polish), Replay one click away. Never a broken or empty editor pane.

## Honesty rules (design invariants)

1. Live output is labeled "agent step — gates run in the CLI" wherever it renders.
2. No fabricated attribution/mapl for live output: those panes show the reference session's data with a "from the recorded generation" tag, or nothing.
3. Objectives never gate on live-generated code content. `generation-ran(live)` gates on the *act*, not the artifact.
4. The reference solution is always the recorded real session; live output renders beside it, divergence is surfaced, lesson 8 teaches why that's the point.

## WebContainers — the test gate, in v1 (user decision 2026-07-24)

WebContainers (StackBlitz, what svelte.dev's tutorial runs on) provides the one loop piece the wasm core cannot: running the generated TypeScript's vitest suite in-browser. This is in v1 because the test gate is the language's central claim — "iterates until the declared tests pass" — and lesson 8's argument only lands if the visitor can SEE the same declared tests pass against two different generated implementations (the reference and their local model's output). That is the behavioral-spec thesis made physical.

- **Licensing** (checked 2026-07-24, webcontainers.io/enterprise): free for open-source/non-commercial; commercial license required for for-profit production. NAPL is OSS — same basis svelte.dev ships on. Revisit before any commercialization.
- **Headers**: cross-origin isolation (COOP/COEP) required for SharedArrayBuffer. Cheap for this site — fully self-contained (local fonts, no third-party embeds). Set on the Cloudflare worker responses AND the vite dev/e2e servers (`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`).
- **Boot strategy**: lazy — the container boots only when a lesson exposes "Run the tests" (parts 2 and 4); never on page load. Mount a **prebuilt snapshot** (vitest + deps preinstalled, built at site build time) instead of running npm install in-browser; per-run we mount only the generated module files over the snapshot. Boot + first run target < 10s on a warm cache; a progress state covers it.
- **Where it appears**: `run-tests` affordance in lessons 6–8 (reference tests vs live output) and lesson 13 (drifted code still passes tests — why drift is about provenance, not correctness). The test-results panel renders vitest's structured output (pass/fail per declared test, mapped back to the frontmatter `tests:` entries by name — closing the loop visually).
- **Objectives**: lesson completion NEVER gates on the local model's output passing tests (small-model frustration trap). New check kind `tests-ran` (the act), used where pedagogy needs it. Green tests are celebrated; red tests get a "this is what the CLI's gate would catch — the agent would retry" callout, which is itself the lesson.
- **Fallback**: browsers without SharedArrayBuffer/isolation get the recorded test-run output of the reference session with a "recorded" tag, and the affordance explains why. Never a broken panel.

---

# Part III — Delivery

## Also in scope (user-directed, same delivery round)

- Landing: canvasui sprinkle — client-only mount after hydration, static hero fallback, landing route only (user override of prior agent verdict; SSR risk neutralized by the mount strategy). Pick 1–2 effects max; the bar is "distinctive", not "busy".
- Playground copy: button says "Watch it generate" when the engine is replay, "Generate" only when a live model is loaded; replay empty-state says it replays the real recorded generation.

## Docs consolidation (13 → 9)

- Start here: What is NAPL · Quickstart (keep).
- Tutorial entry in the sidebar links to `/tutorial`.
- Guides (3): Writing prompts that hold (includes the enumeration-vs-imperative lesson from the deps slice) · Drift, moves & reconcile · Running it for real (watch, agent engines, CI).
- Reference (2): File formats (.napl + .mapl, every key) · CLI (all commands).
- Self-host (keep). Existing thinking/format/cli pages merge in; no content invented, only consolidated.

## Phasing (ALL v1 — user decision 2026-07-24: nothing deferred. Phases are build order, not scope tiers; every phase ships before the tutorial is called done)

1. **Shell + lesson engine + part 1** — routes, rail, ObjectiveBar, checker (unit-tested), build-tutorial.mjs with solution-must-pass enforcement, lessons 1–4 (engine mode none). e2e: complete lesson 2 by typing the fix; solve path; progress persists across reload.
2. **TransformersEngine + picker** — worker, catalog, guard, GenEngine wiring into PlaygroundShell globally. e2e with a mocked worker (fixture responses; CI never downloads models); manual verification with a real model before commit.
3. **WebContainers test gate** — COOP/COEP on dev/e2e/deploy configs, lazy boot, prebuilt snapshot, run-tests affordance + results panel mapped to frontmatter tests, recorded-output fallback. e2e: mocked container in CI, real boot verified manually before commit.
4. **Parts 2–4 + session fixtures** — 12 lessons, recorded sessions built via build-tutorial.mjs. e2e: replay lesson, live lesson against mocked worker, test-run lesson, mapl objective flow.
5. **Docs consolidation + landing round** — canvasui sprinkle, button relabel, /selfhost mobile stacking (clears the deferred audit finding). Full-site screenshot pass, impeccable-audit sweep on the tutorial.

## Testing summary

- Checker: exhaustive unit tests (every kind × pass/fail).
- build-tutorial.mjs: solution-passes-own-checks is a build gate; idempotence (run twice, hash-compare) like build-fixtures.
- e2e additions (~8 specs): lesson completion by edit, solve, reset, progress persistence, replay generation, live generation (mocked worker), engine picker states, prompt-too-large state.
- All existing site gates remain (typecheck, unit, e2e, build, screenshot pass at 2 widths × 2 themes).

## Risks

- Sub-1B output quality — neutralized by honesty rules (objectives never gate on it; replay is the reference).
- Model download weight — opt-in, metered guard, 250MB floor, cached.
- WebGPU coverage — wasm fallback everywhere, slower, labeled.
- Scripted-session lessons (10–12) could read as fake-live — mitigated in prose wording and the "from the recorded generation" tag; review this specifically in the screenshot pass.
- Scope — phase 1 alone is a shippable tutorial; nothing later blocks it.
