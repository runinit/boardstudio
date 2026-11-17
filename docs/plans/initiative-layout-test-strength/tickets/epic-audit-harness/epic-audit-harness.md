---
type: epic
title: "Audit harness"
status: done
parent: initiative-layout-test-strength
covers: [R7, R8]
sources: []
companions: []
after: []
refined: true
risk: low
---

# Audit harness

## Description

One required test never runs because it needs an external fixture, and Necessist cannot see the Core integration tests at all. This epic gets linked key resize covered in the ordinary gate and proves, with a pilot audit of `core/tests/matrix_placement.rs`, whether a Necessist audit of those tests is trustworthy and worth its cost. Parent: the initiative's Requirements R7 and R8.

## Outcome

Linked keycap-size reflow is checked through Core and history in the ordinary gate, and the team can decide from measured results whether a wider Core integration audit is worth running. That audit is not a prerequisite for refactors.

## Requirements

- R1 (parent R7): The ignored external-archive linked key-resize test is replaced with a synthetic in-code document that runs in the ordinary gate without an environment variable. It checks source size change, neighbor reflow, linked-partner sizes and reflected poses, stable partner identities, and both halves' relevant state through Undo/Redo.
- R2 (parent R8): `core/tests/matrix_placement.rs` has been audited by Necessist with proof that its named tests ran, a deliberate failing control was detected, ignored tests and assertion-helper calls are excluded from the results, each reported finding reproduces in the normal `integration` target, and the measured cost and triage are recorded.

## Constraints

- No production refactor or behavior change belongs to this epic. A newly failing repaired test is evidence for a separate bug investigation, not proof of a product defect.
- Necessist measures sensitivity to deleting test statements, not production mutations. Neither a clean pilot nor completion of a wider audit is a prerequisite for other work.
- Validate target mapping and detect a deliberate failing control before accepting pilot results. Record the named tests' execution and ignored counts; a filtered-out or ignored test is not a passing behavioral result.
- Keep temporary target mapping, audit scaffolding, mutations and databases out of commits. Preserve the project's normal `integration` target and test layout.

## Done when

- No ignored linked key-resize test remains; its synthetic replacement runs without an environment variable and checks source size/reflow, linked-partner sizes/poses and identities, and both halves' relevant state through Undo/Redo.
- `core/tests/matrix_placement.rs` has an audit with validated target mapping, proof that the named tests ran and a detected deliberate failing control; ignored tests and assertion-helper calls are excluded from its results.
- Each finding from that audit reproduces in the normal `integration` target.
- A written result in this epic's Notes records removals per hour, findings, and a recommendation on a wider audit.
- The full project check passes on the delivered branch.

## Boundaries

- Out: repairing the seven named tests; owner: epic-repair-weak-tests.
- Out: auditing any other Core integration file.
- Out: restructuring the tracked Core integration tests.
- Out: delivering audit scaffolding; temporary `[[test]]` entries or a scratch crate are never committed, and neither is any Necessist database.
- Must not change: how the `integration` binary is built and run by the project check.
- The pilot audit runs at a recorded commit in its own worktree with its own database, because Necessist rewrites test files in place while it runs.
- The pilot checkout and execution require separate authorization; this planning approval creates no checkout and runs no audit or build.
- An unsuccessful pilot records a blocker and does not satisfy R2/initiative R8 or close this epic.

## References

- parent — docs/plans/initiative-layout-test-strength/initiative-layout-test-strength.md, section Requirements
- research — docs/plans/initiative-layout-test-strength/research-necessist-audit/research-necessist-audit.md, sections Lead, Harness limitation, Review corrections

## Notes

- Decision: 2026-11-16 — user explicitly approved epic closure after the resumed assessment. Both 2.1 and 2.2 are accepted/done; recorded implementation, controls, normal-target replays, measured recommendation and passing six-step checks satisfy R1/parent R7, R2/parent R8 and Done when. A1–A3 are completed and A4 applied; acceptance commit `a4a5552647ed968dba04e52d6aa65f6520e24149` accepts A2 and pilot 2.2. Earlier open/pending-acceptance summaries below are historical and superseded. No new verification, wider audit or cleanup ran. Whole-initiative closure remains a separate confirmation.

- Decision: 2026-11-16 — user scheduled retrospective A1–A3 as independent backlog follow-ups, without changing this epic's accepted Requirements/Done when or starting implementation: A3 (backlog 2, preserve pilot reconstruction evidence) has working priority before disposable cleanup; A1 (backlog 3, linked metadata oracle) follows; A2 (backlog 4, nonzero linked rotation) waits on A1 in the shared test lane. A3 is technically independent. Sofia applied A4's historical-state labels to the earlier pilot-status note and story 2.1's delegate handoff; no ticket status or frozen intent changed. Spike 2.2 remains built and the epic remains open pending explicit human acceptance/closure.

- Evidence: 2026-11-16 — approved spike 2.2 pilot ran in host-created `/home/chris/.local/share/opencode/worktree/3ca6f9/integration-audit-pilot-2-2` at `b8d7011a259736ec4838161a095fc6aee0779706`, temporary standalone target only. Both standalone and ordinary integration targets executed the same 29 named tests with zero ignored (normal target: 874 filtered). Vector/clear/assert control failed in both targets when clear was deleted; Necessist detected that failed statement removal and captured the diagnostic before creation of a fresh disposable-only DB. Six assertion-helper names and three assertion-closure-bearing tests were excluded from removals; the latter still ran in baseline verification. Two additional helper-only tests have zero eligible candidates. Delivered mapping and production remain unchanged.
- Result: 2026-11-16 — 165 pilot removals in 69.09 s: 27 passed, 29 failed, 109 nonbuildable, zero timed-out/skipped. Throughput: 8,597 completed candidates/hour including nonbuildable, or 2,918 buildable outcomes/hour. Approximate setup wall time 368.29 s (includes inspection/calibration); setup plus audit 437.38 s gives 1,358 candidates/hour. Initial standalone/integration named build-runs took 0.55/24.90 s; no isolated cold-cache benchmark. Individual normal-target replays plus restored module pass took another 140.29 s. Every passing span matched baseline and reproduced individually in normal integration; restored source passed both targets again. Triage: 14 benign redundant operations, 13 fixture-sensitivity leads without an established defect, zero stale/harness-only results, **zero confirmed findings in one file**. Diagnostics confirmed original fixtures exercise their intended nondefault state; no repairs made.
- Recommendation: 2026-11-16 — pilot mapping/control are trustworthy for eligible candidates, not blanket integration coverage. Do not start a wider audit without a separate decision: 109/165 outcomes were nonbuildable and 27 passing removals produced no confirmed defect after normal-target triage. Prefer separately approved small high-risk files, candidate inspection, explicit helper/closure gaps and setup-inclusive cost. Evidence and complete triage are in [the spike plan](tickets/spike-prove-and-calibrate-the-integration-audit-pilot-plan.md); logs are under repository-root `.evidence/integration-audit-pilot-2-2/` (`pilot.log`, `calibration.log`, `cargo-*.log`, `replay-*.log`, `fixture-diagnostics-serial.log`, `final-restored.log`). Parent DB and retrospective left unread/untouched; temporary mapping, DB and scratch stayed disposable/uncommitted. Full project check and Build review are recorded separately in that plan; epic remains open pending acceptance.

- Decision: 2026-11-16 — user accepted story 2.1, implemented in `92ac157c2de936b71df297a60212fd7b3e274e67`; its plan is now done. Evidence records one named pass with zero ignored, ten module passes, four detected/restored counter-examples, all six full-check steps passing and no quick-review findings. Historical state at that acceptance decision (superseded): “The epic remains in progress for spike 2.2, whose disposable checkout, fresh database and audit execution still require separate authorization; no pilot has started.” The user subsequently authorized that setup/execution; the completed pilot and Build evidence are recorded above and in spike 2.2's plan (`086fa1f21725e3868460a66b328328b9ad5fdb13`). Spike 2.2 remains built pending human acceptance; this historical label does not close the epic.

- Decision: 2026-11-16 — refined from the initiative's R7/R8 and the approved read-only continuation; no further shared design decision is needed. Story 1 owns reproducible linked-resize coverage, and spike 2 owns the uncertain integration-audit route and bounded pilot.
- Decision: 2026-11-16 — the tracer bullet is story 1: a small synthetic linked document reaches real Core acceptance, source keycap-size reflow, linked propagation and history in the ordinary gate. This removes the external-fixture dependency before the less certain harness pilot.
- Evidence: 2026-11-16 — source inspection, not execution proof: `accepted_rotated_and_mirrored_resize_preserves_assemblies_through_history` in `web/crates/layout/src/objects/keycap_resize.rs` is single-matrix coverage, including geometric mirroring, not linked-half propagation. `typed_keycap_size_writes_sizes_moves_and_offsets_in_one_undo_step` in `core/tests/keycaps.rs` has no linked partner. The linked resize cases in `core/tests/matrix_anchors.rs` change matrix dimensions through `SetMatrix`, not keycap sizes. None makes the ignored linked keycap-size reflow case redundant.
- Decision: 2026-11-16 — replace `replace_document_reflows_source_and_core_syncs_linked_partner_through_history` with a synthetic in-code fixture, retaining its source reflow, partner sizes/reflected poses and stable-identity checks. Extend the history oracle to both halves' relevant state; the old test checked only the selected source keycap after Undo/Redo. Do not add an archive fixture, environment-variable dependency or ignored replacement.
- Decision: 2026-11-16 — story 1 and spike 2 have no technical prerequisite on each other or the repair epic. Table order records the chosen sequence, repairs then reproducible coverage then the pilot; do not encode that preference as an `after` gate. Calibration and its failing control precede the bounded pilot inside spike 2.
- Decision: 2026-11-16 — the pilot adapts to the existing Core test layout, at a recorded commit in a separately authorized disposable checkout and database. Record setup/elapsed time, removal outcomes, removals per hour, findings per file and triage in this epic's Notes, with log references. If target mapping or the failing control cannot be established, report the blocker without treating R2 as delivered. No wider audit is authorized by this ticket.
- Decision: 2026-11-16 — the continuation examined 40 web removals with no additional confirmed defect: 29 active removals without an established new defect and 11 already belonging to L1. The optional rotated-resize size/reflow lead remains unconfirmed and gets no new ticket. Do not revive the deleted draft audit plan or any canceled plan; no production-refactor ticket is justified.
