# Removing tests from frontmatter: the prompt is the prompt

Date: 2026-07-25. Status: DRAFT rev 3, after two adversarial review rounds (gpt-5.6-sol r1: 2 critical, 5 high, 5 medium, rethink; r2: 11 of 12 resolved, 1 high 1 medium 1 low remaining, sound with fixes; all applied below). Planned only, nothing scheduled, no implementation until approved and sequenced.

## The problem

Frontmatter was meant to carry identity and wiring: `module`, `targets`, `deps`. `tests:` snuck behavior into it, and behavior data rots into implementation shape. The evidence is any mature selfhost prompt: `reconcile_derive` carries given/expect blocks full of `reason: edited`, `current: null`, `reconcile_files: []`. That is a serialized unit test wearing YAML, not a specification a human writes to another human. Three concrete failures:

1. Leak. The given/expect vocabulary mirrors internal types. Prompts couple to the implementation's data shapes, which is exactly what killed `src:` paths.
2. Duplication. The prose must state the behavior anyway; the tests restate it as data. Two sources of truth in one file.
3. Authorship distortion. Humans write worse prose when a test block carries the load. The prompt drifts from specification toward test fixture.

The principle: the prompt is the prompt. Prose specifies behavior completely. Everything derived from it belongs to the machine layer.

## What tests: does today, honestly

- Drives the gen test gate: the agent iterates until the cases pass or the gen fails after MAX_ATTEMPTS. Load-bearing.
- Pins data-shaped behavior exactly, which prose does imprecisely.
- Constrains the code-writing agent INDEPENDENTLY: the cases exist before the agent runs and the agent cannot weaken them. This third property was under-weighted in rev 1 and is the crux of the review's first critical: any design where the code-writing agent also authors the checks is a self-grading loop.

## The design: derive, sanction, freeze, gate

Prose is the sole human-authored source. The test corpus becomes machine state under an explicit independence and sanctioning protocol.

### Phase 1: derivation (independent of code)

At gen time, BEFORE any implementation exists in context, a derivation step produces the proposed corpus from the prompt prose alone: a separate agent invocation with its own context containing the prompt body and the corpus schema, never the current or generated implementation, never the code-writing conversation. The derivation is attempt-budgeted separately and validated structurally (schema, deterministic ordering, canonical serialization). Separate model optional; separate context and this ordering are mandatory.

### Phase 2: sanctioning (human authority, not a journal entry)

A derived corpus is a PROPOSAL. It becomes authoritative only through explicit acceptance:

- First derivation for a module: gen halts with the proposed corpus rendered for review (cases, prompt-line provenance per case); the human accepts or rejects (`napl gen` interactive prompt, or `--accept-tests` for scripted flows that still record the acceptance in the journal as a human decision).
- Any later derivation that differs from the sanctioned corpus is a classified diff: gen surfaces added/removed/changed cases with their prompt-line provenance and requires the same explicit acceptance. This applies to EVERY regen class, expected-no-op and expected-change alike; rev 1's reliance on the no-op rule alone was wrong because most edits are expected-change and corpus drift would ride along unpoliced.
- A fresh derivation NEVER auto-replaces the sanctioned corpus, regardless of which agent or model produced it. Provenance (model and backend identity, prompt hash, derivation schema version, toolchain version) is stored in the corpus document.

Pipeline: proposed corpus, then reviewed and accepted corpus, then immutable gate input.

### Phase 3: freeze and gate (independence preserved)

The sanctioned corpus is immutable for the duration of code attempts: the code-writing agent receives it read-only as part of its task, exactly as it receives frontmatter tests today, and iterates until the cases pass. The agent cannot modify the corpus mid-attempt; a gen in which the corpus file changed between sanction and gate is a hard failure. This preserves today's independence property with a stronger provenance trail than YAML in the prompt ever had.

### Execution model (what "runs" means)

The corpus is given/expect data plus a per-target CALL BINDING derived and sanctioned with it: the entry function under test, the literal argument mapping, and the assertion form. From that, the TOOLCHAIN renders the native tests itself through a pinned per-target template: pure mechanical codegen, no agent anywhere in the projection. The code-writing agent never materializes tests; it receives the already-rendered tests read-only and makes them pass. This closes the r2 high finding properly: correspondence is not a name check on agent-written tests (a correctly named test could still weaken its body); the toolchain wrote the bodies, so case-to-test fidelity holds by construction, and the validator's only job is byte-comparing the rendered files against the template output. `napl test` runs the rendered tests directly: no inference, no network, deterministic, target-specific execution through the existing cargo/vitest harnesses. Rendered-case failures are distinguished from unrelated project test failures by the render boundary (separate test files owned by the toolchain), with distinct exit classes and byte-pinned diagnostics for conformance. Stale, absent, corrupt, or newer-schema corpus files are hard status errors with pinned messages. A binding the template cannot render (an API shape outside the template grammar) fails the derivation phase loudly, before sanctioning, and is either a prose bug or a template-grammar gap to triage.

### Storage

`.napl/tests/<module>/<target>.yaml`, target-scoped from day one (adopting the attribution v2 lesson; one file per module would recreate the multi-target clobber bug). Versioned schema, byte-pinned serialization, locked like generated sources, journaled. The corpus joins the same atomic generation transaction as source, map, attribution, mapl, and journal: staged, committed under the durable-commit protocol the AST/docs design defines, rolled back preserving the previously sanctioned corpus. This is machine state that participates in correctness and gets the same recovery rigor.

### Amendment (narrow, visible, not a sibling corpus)

Prose-first correction remains the primary lever: a missing case is a prose bug. But rev 1's absolute "no amendment channel" was too brittle for the real cases (a systematically missed stated case, a permanently pinned security regression, a target-specific representation prose cannot carry). The mechanism: a human may PIN an individual case into the sanctioned corpus through the same acceptance flow, with mandatory prompt-line provenance and a reason string; pinned cases are marked, surfaced by `napl status` as standing debt, and re-validated against derivation on every gen. When a derivation starts producing a pinned case naturally, the pin is NOT cleared automatically (that would be an unsanctioned corpus change); the clearing appears as a proposed metadata diff in the same acceptance flow. No free-form sibling test files, ever.

### mapl and ambiguity

The mapl reply gains a `tests` section per gen: cases derived, prompt-line anchors, diffs against the prior sanctioned corpus, pins outstanding. A derivation that is unstable across attempts for unchanged prose surfaces as an ambiguity entry on the offending sentences: the language telling the author to write a better sentence.

## Blast radius (a major version, not a cleanup)

- Frontmatter schema drops `tests:`; presence is a parse error with a migration message. This flip is the LAST step, not the first.
- Toolchain: derivation step, sanctioning flow, correspondence validator, corpus state and recovery, cmd_test successor, journal and mapl schema, status. All prompted modules by then; every change is a prompt edit plus classified regen.
- Conformance: new scenarios for derivation, sanctioning, diff-acceptance, correspondence enforcement, pin lifecycle, corrupt-state refusals, and the frontmatter-to-gate connection itself (the review found today's corpus does not even pin that link explicitly; add those scenarios regardless of this plan's fate).
- Selfhost: roughly 50 prompts carry `tests:`. Migration is NOT mechanical (the review is right): each module's prose must be audited to carry what its block pinned before the block can go.
- Site: tutorial lessons 2 and 4 and two docs pages rewritten. The tutorial must teach the full loop, not hide it: how prose produces exact cases, how to inspect the proposed corpus, how to reject a wrong derivation, why a prose edit changed cases, how to express boundary examples in prose without recreating YAML as awkward sentences.

## Migration: phased validation, single release boundary

1. Shadow phase: derivation runs alongside existing frontmatter tests, module by module; the EXISTING blocks act as the validation oracle (the derived corpus must cover every existing case or the gap is triaged as prose bug versus derivation bug). Nothing is removed; a comparison report accumulates.
2. Sanction phase: per module, once shadow comparison is clean, the derived corpus is sanctioned and the frontmatter block deleted in the same classified regen.
3. Flip phase: when all modules are migrated, the strict schema change lands (tests: becomes a parse error) with the conformance re-spec in the same slice. One release boundary at the end, validation module-by-module before it, never a mixed-mode corpus for months.

## Sequencing

After rust-final and after the AST/docs feature ships, as prompt-only feature three. Confirmed reasons: derived-test provenance display needs the AST sentence layer and docs surface, and the corpus transaction needs the durable-commit machinery AST/docs D2 builds. The plan adopts that design's target scoping and recovery lessons wholesale.

## Open questions for the maintainer

1. Sanctioning UX: interactive accept inside `napl gen`, a separate `napl tests review` command, or both? Recommendation: both. CI never accepts: it inspects and FAILS on pending proposals or pins past the cap; acceptance is exclusively a human, local act.
2. Should the shadow phase's comparison report block at less than 100 percent existing-case coverage, or is a triaged allowlist acceptable during migration? Recommendation: 100 percent or triaged-with-reason, no silent gaps.
3. Derivation backend: pinned to the project's lock.json agent, with provenance recorded and any backend change treated as a corpus-diff event to re-review. Confirm.
4. Does the pin mechanism cap (a maximum pinned-case count per module before status escalates from note to warning)? Recommendation: yes, small, pins are debt.

When approved, this graduates to a full design (corpus schema grammar, derivation task contract, correspondence validator rules, scenario list) and re-enters the review chain before any slice is cut.
