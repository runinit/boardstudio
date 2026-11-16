---
type: initiative
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
- R3 (F3): The non-finite splay-origin case is rejected by the finite-origin check itself, shown by the case failing when that check is absent.
- R4 (F4): The layout-viewer pick test no longer carries the duplicate-reference block that cannot reach duplicate resolution; it still asserts that a unique reference selects its part, and the direct duplicate-rejection test in `case_preview.rs` still passes.
- R5 (F6): The rotated-envelope snap test fails when envelope rotation is ignored.
- R6 (F7): The membership-cache test shows a populated membership cleared on close and recomputed when the same owner reopens, and fails when membership is not retired on close.
- R7 (L1): Linked key resize through Core is covered by a test that runs in the ordinary test gate without an environment variable: either existing coverage is shown and the ignored test is deleted, or the test is rebuilt on a synthetic document constructed in test code.
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
- No ignored linked key-resize test remains, and linked key resize through Core passes in the full project check.
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

- Decision: 2026-11-15 — two epics, test repairs first and audit harness second, so the certain repairs do not wait on the uncertain harness work. One epic of six entries was offered and declined.
- Decision: 2026-11-15 — no platform-baseline epic; this is an existing codebase with its check and build in place.
- Decision: 2026-11-15 — kept outside Layout controls polish; that initiative's tickets are unchanged.
- Decision: 2026-11-15 — options chosen for least maintenance: a failing repaired test lands with its fix under a bug ticket in the repair epic; the duplicate-pick block is removed, not repaired; the linked key-resize test is deleted if covered elsewhere, else rebuilt on a synthetic in-code document; the P1 helper cleanup is in as the repair epic's closing story.
- Decision: 2026-11-15 — the pilot audit file is `core/tests/matrix_placement.rs`, chosen for sample size; nothing from the pilot is committed.
- Decision: 2026-11-15 — the tracked Core integration tests stay as laid out; the audit adapts to them, not the reverse.
- Decision: 2026-11-15 — the draft `docs/plans/plan-core-integration-necessist-audit.md` is deleted; the audit-harness epic's pilot audit replaces it.
