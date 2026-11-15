---
type: epic
title: "Prove real production composition and lifecycle boundaries"
parent: initiative-editor-workspace-extraction
covers: [R1, R2, R3, R4, R5, R6]
sources:
  - docs/plans/initiative-editor-workspace-extraction/initiative-editor-workspace-extraction.md
  - docs/plans/editor-workspace-extraction/issues/002-production-composition-test-seam.md
companions:
  - docs/plans/research-editor-workspace-ownership/research-editor-workspace-ownership.md
after: []
refined: true
risk: high
---

# Prove real production composition and lifecycle boundaries

## Description

Establish the actual production composition as the test seam, preserving startup and behavior while supplying the existing test Runtime. Original 002 is the only story: its scope-correct characterization proves which work and resources belong to App, Editor and visible children before further extraction is considered.

## Outcome

Maintainers can exercise real navigation, reconciliation, pending work and teardown together without validating a simplified second application.

## Requirements

- R1 (parent R1): Tests supply the existing Runtime through one private production-composition seam while normal startup retains its constructor, one subscriber, reconciliation and provider ancestry.
- R2 (parent R2): A scope-correct two-board mounted fixture submits a real edit with filtered reply/save gates, bounded waits and independent saved/accepted assertions, without cross-scope warnings or dropped-Signal errors.
- R3 (parent R3): Actual workspace/header controls preserve surfaces, production board selection reconciliation and Export Back/Escape/header return to the last non-Export workspace; workspace cycling creates no accepted-document edit.
- R4 (parent R4): A held edit survives workspace leave/return with once-only settlement and no obsolete feedback or draft writes; reused-ID project replacement establishes a new session identity and rejects retained stale callbacks, with late-reply rejection explicitly evidenced at its real adapter seam.
- R5 (parent R5): Library hides rather than remounts Editor; actual Editor-only and App removal followed by releasing held work causes no stale DOM, navigation, focus or dropped-Signal access while Session-owned work retains its existing drain/save rules.
- R6 (parent R6): Existing useful tests remain, preferences/global DOM are isolated and restored, normal startup ships no fixture controls/alternate persistence, and evidence records baseline, exact commands, diagnostics and receipt scope.

## Functional acceptance for original 002

1. Production startup retains its Runtime constructor and behavior; tests inject the existing real test-support Runtime through one private seam. (R1)
2. Mounted tests use the production subscriber and actual workspace/header controls. Board navigation advances real scope generation/transition and clears selection under current policy, not a manually incremented generation. (R1, R3)
3. A minimal two-board fixture supports real edit submission, filtered reply/save gates, bounded state waits and saved/accepted assertions; preferences and global DOM/listener state are isolated and restored. (R2, R6)
4. Cycling workspaces preserves expected surfaces and causes no accepted edit. A held field edit can leave/return and settle once without stale feedback or draft writes. (R3, R4)
5. Export entry, Back, Escape and header toggle return to the actual last non-Export workspace; return tracking is not replaced by an alias of the workspace signal. (R3)
6. Library hides rather than remounts Editor while permitted work settles. Actual Editor-only removal and actual App removal exercise real scope teardown, not root-DOM removal or a Close request. (R5)
7. Releasing held work after removal cannot revive DOM, navigation or focus, or access dropped Signals. Session-owned work follows existing drain/save rules instead of cancellation with its observer. (R4, R5)
8. A replacement project reusing document/board/part IDs establishes a new session identity; retained stale callbacks cannot act on it. Name the existing controlled-adapter late-reply rejection proof separately; serialized Open is not post-replacement reply delivery. (R4)
9. Mounted edit, workspace switch, board reconciliation, Export return, gated completion and genuine teardown run without mockup cross-scope warnings or dropped-Signal errors. If safe driving cannot be shown, stop extraction and record the blocker; do not suppress diagnostics. (R2, R3, R5)
10. Normal startup ships no test diagnostics, alternate persistence, mockup feature, fixture UI or explanatory HTML. (R1, R6)

## Constraints

The initiative's constraints bind this epic: preserve hook scope, providers, current behavior, operation-specific admission and shared authority; no controller extraction, redesign, new public interface/schema, new registry or wholesale mockup merge. The fixture uses real in-process Session/Core with memory persistence; that is not packaged-worker, browser-store or GPU qualification. Record pre-existing diagnostics separately, not as a waiver.

## Done when

1. All R1–R6 and the ten original 002 criteria have observable passing proof through real production composition; normal production startup still uses its original Runtime construction and single subscriber.
2. The unchanged-source baseline, current revision, exact commands, critical gate timing, independent oracles, diagnostic classification and receipt scope are recorded. Existing native/feature evidence is reused where sufficient, with its seam named.
3. The fixed boundary completes in this order: genuine RED or named proof-gap obligation → fix → affected checks → boundary mutations → one fresh reviewer covering Spec and Standards → integrated gate. Browser/WASM-only Rust changes include the repository lint check; proof-gap mutations must fail at their named assertion/tier, while RED rows are not redundantly mutated.
4. The integrated 002 gate passes documented changed-scope quick checks, page-only WASM compilation, page-bin mounted browser suite, relevant Runtime tests and retained mounted Editor tests. Record reused versus newly executed results; this subset is not original 013's full pre-PR/provider/drag acceptance.
5. Real Library hiding, Editor-only removal and App removal remain distinct under held-work completion. Unsafe fixture driving/unmount blocks this gate, and normal startup contains no fixture-only controls or mockup diagnostics.

## Boundaries

- Own all six initiative requirements and only original 002; no sibling epic or cross-epic prerequisite is required.
- Consume existing Runtime, Session/Core support, controlled reply/save adapters, production providers, navigation and cleanup resources. Refresh current callers and baseline before implementation, preserving unrelated work.
- Preserve all six workspace behaviors and current accepted UI policies. Exercise their controls; do not relocate their controllers or impose obsolete dock/panel policies on newer approved UI.
- Do not implement original 003–013, new camera math, broader draft retention, firmware/package work, browser harness replacement or a new lifecycle framework.
- Completing this epic does not close original 001 or 013, change any other plan, or prove real-worker/GPU qualification.

## References

- parent — [initiative](../../initiative-editor-workspace-extraction.md), Requirements R1–R6 and Constraints.
- source — [original 002](../../../editor-workspace-extraction/issues/002-production-composition-test-seam.md), entire scope, acceptance and verification.
- source — [original map](../../../editor-workspace-extraction/map.md), shared implementation and verification contract.
- proposal — [approved phase and original dispositions](../../../research-editor-workspace-ownership/ticket-proposal.md), Approved scope and Requirements and source.
- research — [ownership report](../../../research-editor-workspace-ownership/research-editor-workspace-ownership.md), Questions 1–4 and limits; source inspection is not passing runtime evidence.
- process — [issue tracker](../../../../agents/issue-tracker.md), ordered slice boundary and completion record.

## Notes

- Decision: 2026-11-15 — user approved this one-epic phase and original 002 as its sole immediate frontier; do not create or repurpose an implementation identity for parent 001.
- Decision: 2026-11-15 — tracer is original 002: real edit → held save → workspace transition → accepted publication → actual owner removal. A designer sees the same production surfaces and return destination, and no obsolete UI revival after completion.
- Decision: 2026-11-15 — no fabricated human setup; hitl is false. Existing application/test support is reused, not a new scaffold or architecture decision.
- Decision: 2026-11-15 — no extra refactor sweep; the source ticket needs no workspace controller move. Original 003–013 retain their source identities, blockers and unresolved dispositions outside this executable phase.
- Risk: high because stale ownership or teardown mistakes could misdirect edits or touch dropped state. Additional verification beyond the new examples: retain and run existing mounted Editor and relevant Runtime suites named by the source, not only the new composition cases.
