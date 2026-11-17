---
type: initiative
status: done
title: "Layout engine and PCB layout tests fail when the behaviour they name breaks"
parent: none
sources:
  - docs/plans/initiative-layout-test-strength/research-necessist-audit/research-necessist-audit.md
companions: []
after: []
risk: low
---

# Layout engine and PCB layout tests fail when the behaviour they name breaks

## Description

A Necessist audit and an independent review found seven tests in the Layout engine and PCB layout areas that cannot detect the regression their names describe (one of them through a block that duplicates a direct test), one opt-in test that never runs, and no way to audit the Core integration tests as they are laid out. When this is done, six of those tests fail under their counter-examples, the redundant block is gone, linked key resize is covered in the ordinary gate, and there is a measured answer on whether auditing the Core integration tests is worth doing. Source: the research document under References.

## Outcome

Anyone changing matrix layout, constraint, snapping or layout-viewer code gets a failing test when they break the named behaviour, where today seven tests would stay green.

## Requirements

- R1 (F1): The case-closure adoption test fails when case-closure parts are no longer excluded from adoption, and shows an ordinary unassigned part being adopted in the same fixture.
- R2 (F2, F5): The persisted framed-constraint test and the local-mirror endpoint test each fail when the solve does not move the target.
- R3 (F3): The non-finite splay-origin case uses a fresh otherwise-valid matrix and rejects the invalid origin; temporarily replacing only that origin with a finite value makes the unchanged rejection assertion fail. This proves dependency on invalid origin, not detection of a single finite-origin guard removal.
- R4 (F4): The layout-viewer pick test no longer carries the duplicate-reference block that cannot reach duplicate resolution; it still asserts that a unique reference selects its part, and the direct duplicate-rejection test in `case_preview.rs` still passes.
- R5 (F6): The rotated-envelope snap test fails when envelope rotation is ignored.
- R6 (F7): The membership-cache test asserts concrete nonempty eligible and live membership immediately before close, after its owner-change cases; close clears both, and reopening the identical full owner recomputes membership. The test fails when its close step is bypassed.
- R7 (L1): The ignored external-archive linked key-resize test is replaced with a synthetic in-code document that runs in the ordinary gate without an environment variable. It checks source size change, neighbor reflow, linked-partner sizes and reflected poses, stable partner identities, and both halves' relevant state through Undo/Redo.
- R8 (H1, C2, C4): `core/tests/matrix_placement.rs` has been audited by Necessist with proof that its named tests ran, a deliberate failing control was detected, ignored tests and assertion-helper calls are excluded from the results, each reported finding reproduces in the normal `integration` target, and the measured cost and triage are recorded.
- R9 (P1): The repeated resize/reset/zip fixture block in the three test modules P1 names is replaced by one helper per module, with scenario inputs and assertions unchanged.

## Constraints

- No production behaviour changes inside a repair story. A repaired test that fails against current production code gets a bug ticket in the same epic, and the repaired test lands together with that fix; no ignored test is committed (C3).
- No production refactor is in this initiative (C5).
- A Necessist run is evidence, never a gate: no ticket here makes one a prerequisite for other work (C1).
- The tracked Core integration tests are not restructured to suit Necessist.
- A counter-example is a temporary change that breaks the named behaviour, in production code or in the test's own act step; it is run to show the test fails, recorded in the story's plan, and never committed (C1).

## Done when

- Each of the six repaired tests (R1, R2, R3, R5, R6) fails when its counter-example is applied and passes without it; the redundant duplicate-pick block (R4) is removed.
- The repeated resize/reset/zip fixture block is one helper per test module, with scenario inputs and assertions unchanged.
- No ignored linked key-resize test remains; the synthetic replacement checks source reflow, linked-partner state and both halves through Undo/Redo, and passes in the full project check.
- A written result states whether a wider Core integration audit is worth running, with measured removals per hour and findings per file.
- The full project check passes on the delivered branch.

## Boundaries

- Out: production refactors of the Layout engine or outline code.
- Out: wasm-only tests, PCB wiring and electrical tests, mirrored-half reconciliation tests.
- Out: auditing any Core integration file other than `core/tests/matrix_placement.rs`; a wider audit is a later decision made from R8's result.
- Out: any other test-helper or fixture cleanup beyond the block P1 names.
- Must not change: the behaviour each test names; existing assertions are not weakened or removed, except the duplicate-reference block R4 removes. Fixtures change and assertions may be added.
- Tracer path: repair one Core fixture test end to end (R1) with its counter-example shown failing, then repeat the pattern for the rest, then the harness.

## References

- research — docs/plans/initiative-layout-test-strength/research-necessist-audit/research-necessist-audit.md, sections Findings, Lead, Harness limitation, Review corrections

## Notes

- Decision: 2026-11-16 — user explicitly approved initiative closure and commit after the completed closure check found no unmet declared criteria. All seven tickets and both epics are done. Recorded controls/restorations cover R1–R6, the helper sweep covers R9, and accepted linked coverage and the calibrated pilot cover R7–R8; recorded full project checks pass. Audit-harness closure and retrospective reconciliation are committed in `66214d4`. This closure uses existing evidence, with no new builds, tests or controls. Historical browser failures and their unproven causes remain preserved; wider audits and disposable cleanup require separate decisions. Protected pre-existing untracked files remain unread and untouched.

- Decision: 2026-11-15 — two epics, test repairs first and audit harness second, so the certain repairs do not wait on the uncertain harness work. One epic of six entries was offered and declined.
- Decision: 2026-11-15 — no platform-baseline epic; this is an existing codebase with its check and build in place.
- Decision: 2026-11-15 — kept outside Layout controls polish; that initiative's tickets are unchanged.
- Decision: 2026-11-15 — options chosen for least maintenance: a failing repaired test lands with its fix under a bug ticket in the repair epic; the duplicate-pick block is removed, not repaired; the linked key-resize test is deleted if covered elsewhere, else rebuilt on a synthetic in-code document; the P1 helper cleanup is in as the repair epic's closing story.
- Decision: 2026-11-15 — the pilot audit file is `core/tests/matrix_placement.rs`, chosen for sample size; nothing from the pilot is committed.
- Decision: 2026-11-15 — the tracked Core integration tests stay as laid out; the audit adapts to them, not the reverse.
- Decision: 2026-11-15 — the draft `docs/plans/plan-core-integration-necessist-audit.md` is deleted; the audit-harness epic's pilot audit replaces it.
- Decision: 2026-11-16 — R3 adopts the user-approved test-act control: replace only the invalid splay origin with a finite value and demonstrate failure of the unchanged rejection assertion. Redundant downstream validation means this does not prove detection of a single input-guard removal. Only repair entry 1.3 needs revised acceptance; no requirement ids or epic ownership change.
- Decision: 2026-11-16 — the repair epic records the existing nearest-previous-heading policy and the test-local fixture correction for 1.3; the observed 360-degree mismatch does not establish a production defect. No bug or policy spike is added. Stories 1.1/1.2, their plans and built status stay unchanged. Build remains paused; this is a planning-only replan, not approval to resume implementation.
- Decision: 2026-11-16 — approved audit continuation clarifies R6 and entry 1.4: seeding keys at the start is insufficient because the owner-change cases switch to an empty board. Require concrete nonempty eligible and live membership immediately before close, then reopen the identical full owner. Keep the redundant-pick-block removal in that entry and the test-only helper sweep as entry 1.5.
- Decision: 2026-11-16 — source inspection found no equivalent active coverage for R7's linked keycap-size reflow and history contract. Rebuild the ignored test on a synthetic in-code document, rather than delete it as redundant, and check both halves' relevant state through Undo/Redo. This supersedes the earlier either/or delivery choice without changing the requirement id or epic ownership.
- Decision: 2026-11-16 — refine the audit-harness epic into story 2.1 and spike 2.2. Schedule 1.4, 1.5, 2.1, then 2.2; retain 1.5's prerequisites on 1.1–1.4, but add no artificial dependency between the harness entries or on the repair epic. An unsuccessful pilot records its blocker without satisfying R8 or closing the epic; it never becomes a prerequisite for other refactors.
- Decision: 2026-11-16 — this approval permits planning-file writes only. Preserve 1.1–1.3, all build plans, existing source edits, canceled plans, the deleted draft audit plan and the active initiative. Do not resume Build or create the future pilot checkout here; checkout creation and execution need separate authorization. Stop for separate approval before any commit.
