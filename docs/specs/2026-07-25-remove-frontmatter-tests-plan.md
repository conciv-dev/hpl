# Removing tests from frontmatter: the prompt is the prompt

Date: 2026-07-25. Status: DRAFT FOR DISCUSSION, planned only, nothing scheduled. Authored on the maintainer's directive: frontmatter `tests:` is the next leaky abstraction on the removal list, same family as the removed `src:` paths. No implementation, no prompt edits, no schema changes until this plan is approved and sequenced.

## The problem

Frontmatter was meant to carry identity and wiring: `module`, `targets`, `deps`. `tests:` snuck behavior into it, and behavior data rots into implementation shape. The evidence is any mature selfhost prompt: `reconcile_derive` carries given/expect blocks full of `reason: edited`, `current: null`, `diff: "PRERECORDED DIFF"`, `reconcile_files: []`. That is not a specification a human writes to another human; it is a serialized unit test wearing YAML. Three concrete failures:

1. Leak. The given/expect vocabulary mirrors internal types (editable sets, reconcile file records, diff fallbacks). Prompts become coupled to the implementation's data shapes, which is exactly what killed `src:` paths.
2. Duplication. The prose must state the behavior anyway (it is the spec); the tests restate it as data. Two sources of truth in one file, and the data half wins fights it should not win.
3. Authorship distortion. Humans write worse prose when a test block will carry the load. The prompt drifts from specification toward test fixture, and reading it cold (the whole point of NAPL) gets harder, not easier.

The principle: the prompt is the prompt. Prose specifies behavior completely. Everything derived from it belongs to the machine layer, not the source.

## What tests: does today, honestly

- Drives the gen test gate: the agent iterates until every case passes or the gen fails loudly after MAX_ATTEMPTS. This is a load-bearing gate, not decoration.
- Pins data-shaped behavior byte-exactly (a given input maps to an expected output or error), which prose does imprecisely.
- Travels with the prompt, so regens are held to stable behavior across time and across agents.

Any replacement must preserve all three properties. The gate cannot weaken.

## Options considered

A. Tests move to prose worked examples only. The agent writes whatever tests it likes; the gate becomes "the agent's own tests pass." Rejected: property 2 dies (nothing pins exact values) and property 3 dies (each regen invents new tests; stability across agents is gone).

B. Tests move to a sibling source file (`<module>.tests.yaml`). Rejected: this relocates the leak without fixing it. Two source files per module, the second still implementation-shaped, still human-maintained. The `src:` removal did not move the paths to a sibling file; it deleted the abstraction.

C. Tests become machine-derived, human-sanctioned state. The prose is the sole source. At gen time the toolchain has the agent derive an executable test corpus FROM the prose, records it in `.napl/tests/<module>.yaml` (machine-owned state, like ir and mapl), runs it as the test gate, and reports the derivation in the mapl reply. Sanctioning rides the existing disciplines: the derived corpus is journaled, diffs of it surface like patches, an expected-no-op prompt edit that changes the derived corpus is a halt (the no-op rule already polices exactly this shape of surprise), and drift in derived tests against unchanged prose is a compile error, the same posture as attribution. Recommended.

Option C preserves the three properties: the gate still runs real tests; exact values are still pinned (in state, journaled, diffable, byte-stable across no-op regens); stability across regens is enforced by the no-op rule instead of by YAML in the prompt. And it fixes the three failures: nothing implementation-shaped lives in source; prose is the single source of truth; prompts read as specifications again.

The philosophical payoff is real: today a human writes tests to check the machine. After this, the human writes meaning and the machine proposes the checks, which the toolchain holds stable and the human can inspect in the margin. That is the mapl model applied to testing, and it is more NAPL than what we have.

## Design sketch (C), to be specified fully before any slice

- Frontmatter schema drops `tests:` entirely; presence is a parse error with a migration message (deny-unknown-fields posture, no deprecation shim, pre-1.0).
- Prose gains a convention, not new syntax: behavior stated with worked examples where exactness matters. The existing pinned-strings discipline (error bytes, thresholds) already does this in the shell prompts; it generalizes.
- New machine state `.napl/tests/<module>.yaml`: the derived corpus, versioned schema, byte-pinned serialization, one file per module, written by gen, never hand-edited (locked like generated sources).
- Gen pipeline: derive corpus from prose (attempt-scoped, shares MAX_ATTEMPTS), run corpus, iterate agent until green, journal the corpus alongside patches. A prompt edit classified expected-no-op must produce a byte-identical corpus or halt.
- mapl gains a `tests` section per gen: what was derived, from which prompt lines (line-range anchors, same as attribution granularity). Ambiguous prose that yields an unstable corpus surfaces as an ambiguity entry, which is the language telling the author to write a better sentence, which is the point.
- `napl test` (or cmd_test's successor) runs the recorded corpus without a gen, for humans and CI.
- The tutorial and docs teach prose-first speccing; the "tests are data, and they gate" page inverts to "the machine derives the tests, and they gate."

## Blast radius (why this is a major version of the language, not a cleanup)

- Schema: schemas_lock untouched; the prompt/frontmatter schema module and its strict parser change, with the parse-error migration path.
- Toolchain: the gen loop's test gate, cmd_test, journal entries, mapl schema, status. All prompted modules by then; every edit is a prompt edit plus regen under the classification discipline.
- Corpus: every conformance scenario whose fixture prompts carry `tests:` changes bytes; scenarios asserting test-gate behavior are re-specified around derivation. This is the sole-spec surface, so this is where the real review lives.
- Selfhost: roughly 50 prompts carry `tests:` today. Migration is mechanical per module (delete block, ensure the prose already pins what the block pinned, expected-change regen, derived corpus must reproduce the old cases or the gap is a prose bug to fix), but it is 50 regen cycles of careful reading.
- Site: tutorial lessons 2 and 4 teach frontmatter tests and must be rewritten; docs writing-prompts and file-formats pages likewise.
- Interaction with the AST/docs plan: that plan already dropped frontmatter-tests-as-Examples, which ages well here. But its scenarios and its Slice C lock-schema work assume today's frontmatter; whichever ships second rebases on the first. Recommendation: this ships AFTER the AST/docs feature, as prompt-only feature number three, because the AST parser and the docs surface make derived-test provenance displayable, and because two concurrent frontmatter-adjacent migrations is how state files get hurt.

## Open questions for the maintainer

1. Sequencing confirmation: after rust-final and after AST/docs, as prompt-only feature three?
2. Does the derived corpus accept human amendments at all (a sanctioned-additions file), or is the only lever the prose? Recommendation: prose only; an amendment channel recreates the leak with extra steps.
3. Should derivation stability across DIFFERENT agents (claude vs codex preset) gate anything in v1, or is per-project agent pinning (already in lock.json) enough? Recommendation: lock pinning is enough.
4. Migration mode: one big wave, or module-by-module opportunistically as prompts are next touched? Recommendation: one planned wave, because a mixed-mode corpus splits the conformance spec in two for months.

When approved, this draft graduates to a full design (grammar of the derived-corpus file, exact gate semantics, scenario list) and enters the standing review chain before any slice is cut.
