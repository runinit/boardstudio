---
type: epic
title: "Repair confirmed weak tests"
parent: initiative-layout-test-strength
covers: [R1, R2, R3, R4, R5, R6, R9]
sources: []
companions: []
after: []
refined: true
risk: low
---

# Repair confirmed weak tests

## Description

Seven tests named by the audit pass whether or not the behaviour they describe works. This epic changes six of them so each fails under its counter-example, removes the seventh's redundant block, and closes by replacing the repeated fixture block with one helper per module. Parent: the initiative's Requirements R1–R6 and R9.

## Outcome

A developer who breaks case-closure adoption, a framed solve, splay-origin validation, duplicate-reference picking, rotated snapping or membership retirement sees the named test fail.

## Requirements

- R1 (parent R1): The case-closure adoption test fails when case-closure parts are no longer excluded from adoption, and shows an ordinary unassigned part being adopted in the same fixture.
- R2 (parent R2): The persisted framed-constraint test and the local-mirror endpoint test each fail when the solve does not move the target.
- R3 (parent R3): The non-finite splay-origin case uses a fresh otherwise-valid matrix and rejects the invalid origin; temporarily replacing only that origin with a finite value makes the unchanged rejection assertion fail. This proves dependency on invalid origin, not detection of a single finite-origin guard removal.
- R4 (parent R4): The layout-viewer pick test no longer carries the duplicate-reference block that cannot reach duplicate resolution; it still asserts that a unique reference selects its part, and the direct duplicate-rejection test in `case_preview.rs` still passes.
- R5 (parent R5): The rotated-envelope snap test fails when envelope rotation is ignored.
- R6 (parent R6): The membership-cache test asserts concrete nonempty eligible and live membership immediately before close, after its owner-change cases; close clears both, and reopening the identical full owner recomputes membership. The test fails when its close step is bypassed.
- R7 (parent R9): The repeated resize/reset/zip fixture block in the three test modules P1 names is replaced by one helper per module, with scenario inputs and assertions unchanged.

## Constraints

- No production behaviour changes inside a repair story. A repaired test that fails against current production code gets a bug ticket in this epic, and the repaired test lands together with that fix; no ignored test is committed.
- A counter-example is a temporary change that breaks the named behaviour, in production code or in the test's own act step; it is run to show the test fails, recorded in the story's plan, and never committed.
- A fixture shared with other tests is changed inside the repaired test, not in the shared helper, so tests outside this epic keep their inputs.

## Done when

- Each of the six repaired tests passes as delivered and fails when its counter-example is applied; the redundant duplicate-pick block is removed.
- The repeated resize/reset/zip fixture block is one helper per test module, with scenario inputs and assertions unchanged.
- A repaired test that fails against current production code has a bug ticket in this epic and lands together with its fix; no ignored test is committed.
- The full project check passes on the delivered branch.

## Boundaries

- Out: the linked key-resize test and the audit harness; owner: epic-audit-harness.
- Out: production code changes, except the fix for a bug a repaired test exposes, under its own bug ticket.
- Must not change: the behaviour each test names; existing assertions are not weakened or removed, except the duplicate-reference block R4 removes. Fixtures change and assertions may be added.

## References

- parent — docs/plans/initiative-layout-test-strength/initiative-layout-test-strength.md, section Requirements
- research — docs/plans/initiative-layout-test-strength/research-necessist-audit/research-necessist-audit.md, section Findings

## Notes

- Decision: 2026-11-15 — refined from the initiative's lines alone; the research file's Findings give each test's location and mechanism, so nothing more is needed before stories.
- Unknown: none blocking. Adoption keeps only parts on the source side of the mirror axis (`core/src/matrix/layout.rs:3612`), so entry 1's hole and ordinary part must both sit there or its counter-example passes for the wrong reason.
- Decision: 2026-11-15 — the fixture helper takes the field it sets (`stagger`, `splay` or `splay_origin`); a module with a single occurrence may use a direct assignment instead. The check is that the resize/reset/zip pattern no longer appears in the three modules.
- Decision: 2026-11-15 — tracer bullet is the case-closure adoption repair (entry 1): it proves the whole pattern once (fixture change, counter-example shown failing, record in the plan, full check) on the finding with the clearest mechanism.
- Decision: 2026-11-15 — the rotated-envelope snap repair is second because it is the repair most likely to expose a production defect.
- Decision: 2026-11-15 — the closing "Refactor sweep" carries R7 (the fixture helper) plus cleanup the earlier stories defer.
- Decision: 2026-11-15 — splitting the sweep into a helper story and a separate sweep was offered and declined.
- Decision: 2026-11-15 — entry 4 keeps the membership repair and the pick-block removal together; the removal is too small to stand alone.
- Decision: 2026-11-16 — the persisted framed solver chooses the equivalent authoritative heading nearest its previous heading. Starting at -20 degrees therefore yields -198.13010235415598 rather than the existing literal oracle 161.86989764584402. For entry 3, start the authoritative target at 0 degrees, still unsolved, while keeping both target positions displaced and the existing exact expected-pose assertion unchanged. This is a test-local fixture correction consistent with existing policy, not a new heading policy or an established production defect; no bug or policy spike is added. Do not replace the assertion with a modulo-angle comparison.
- Decision: 2026-11-16 — R3 and entry 3 use the approved splay-origin test-act control on a fresh otherwise-valid matrix: replace only the non-finite origin with a finite value and show the unchanged rejection assertion fails. Do not disable unrelated finite-result guards or claim this detects removal of a single input guard. Preserve the repeated fixture block for entry 5.
- Decision: 2026-11-16 — keep entry 3 paused during this planning-only replan; do not modify its build plan or source edits here. No stories are added, dropped or reordered; entries 1/2 and their built plans remain unchanged. A separately approved Build resumption must reconcile its existing block with these decisions before continuing the repair, counter-examples, neighboring tests, full check and review.
- Decision: 2026-11-16 — entry 4 must assert concrete nonempty eligible and live membership immediately before close, after the source-change checks, not merely populate its initial fixture. Restore a populated board/instance scope and cache it before closing; reopen with the same scope, accepted token, session epoch, revision, document allocation and scene allocation. Close must not invoke the compute closure, both membership lists must become empty, and same-owner reopen must invoke it again and recover the expected ids.
- Decision: 2026-11-16 — preserve entry 4's existing notification-reuse and source-change assertions, its close-bypass counter-example and the unique-pick/direct duplicate-rejection checks. Entry 5 and its prerequisites on 1–4 remain unchanged; no duplicate repair ticket or production refactor is added. This approval writes planning files only and does not resume any Build or authorize a commit.
