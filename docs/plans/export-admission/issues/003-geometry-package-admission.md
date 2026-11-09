# 003: Geometry-package export admission

Status: ready-for-agent
Type: build
Blocked by: [001: ZMK export admission](001-zmk-export-admission.md)
Related: [Candidate 02 research](../../../research/2026-10-20-export-readiness-dispatch-deepening.md)

## Problem Statement

Keycap STLs, authored Case STEP and generated mechanical packages depend on different accepted geometry and generation evidence. Their Export rows and dispatch paths do not always express the same current prerequisites, while preview meshes are not manufacturing authority. A designer needs to understand which output can begin without mistaking a preview for exact output or letting a retained callback act on a new board.

## Solution

Extend ticket 001's output-specific admission seam to keycap, authored Case STEP and generated mechanical exports. Show a blocker for each output's known current prerequisites, bind clicks to the source that rendered them, and leave exact generation, provider validation, leases and final delivery to their existing owners. Test the real mounted Export flow for each named package.

## User Stories

1. As a designer, I see keycap STL availability for the selected board and current accepted scene rather than from unrelated PCB or firmware readiness.
2. As a designer with eligible keycaps, I can click the mounted row and receive the named STL package from the accepted design.
3. As a designer whose keycap scene changed, I receive no package from stale geometry and see the current prerequisite or failure.
4. As a designer, I see authored Case STEP readiness from authored case geometry and findings, not from the disposable mechanical preview.
5. As a designer with admissible authored Case geometry, I can click STEP and receive the corresponding exact output.
6. As a designer with an authored geometry blocker, I see a specific reason and cannot infer that a visible preview makes STEP admissible.
7. As a designer, I see generated mechanical-package availability for the selected physical instance, mount preparation, exact scene and generation evidence.
8. As a designer whose mechanical preview refreshes during an otherwise authorized export, the refresh alone does not revoke the export's captured ownership.
9. As a designer with missing mounts or generation findings, I see the relevant blocker instead of a misleading Available row.
10. As a designer switching board, physical instance, editor session or accepted revision, a retained geometry-export click cannot generate an artifact for the new owner or deliver stale output.
11. As a designer whose CAD executor or accepted source changes during generation, only a still-current artifact can be delivered and a failure is explained.
12. As a designer with an incomplete split, I can continue saving and editing; unrelated valid outputs remain available while a package needing missing hardware explains its blocker.
13. As a designer whose browser download fails after valid generation, I see a delivery error rather than false success.

## Implementation Decisions

- Reuse ticket 001's output-specific admission seam while keeping distinct accepted-scene, authored-case and generated-mechanical prerequisites. Do not use a universal readiness bit or equate any display mesh with exact manufacturing geometry.
- Use current accepted source and selected board/physical instance for both displayed eligibility and click admission; “Available” is eligibility to begin, not guaranteed generation or delivery. Re-evaluate at click and retain Runtime/Session revalidation throughout work.
- Bind retained callbacks to the source that rendered them. A detached stale row dispatches nothing and reports no false success; a newly rendered row reflects the current owner without needing to message the detached control.
- Keep authored STEP independent of preview currency. Preserve generated mechanical mounts, exact scene and generation diagnostics; do not weaken existing source/worker checks, export leases, active-work ownership or final-delivery validation.
- Leave output construction and provider error reporting with their existing owners; do not change accepted-document authority or incomplete-project save rules.

## Testing Decisions

- **Primary acceptance seam: mounted Export flow.** Click actual Available keycap STL, authored Case STEP and generated mechanical rows with viable accepted fixtures through Runtime/Session. Inspect the named artifact or precise provider limitation/failure; a row-only boolean test or direct Runtime call is not sufficient.
- Mount blocked cases for missing selected board, stale accepted scene, authored Case findings, missing mount initialization, selected-instance mismatch and generation diagnostics. Verify each visible reason and that unrelated ready rows remain actionable.
- Retain rendered controls while changing board/instance, reopening the same ID/revision, advancing accepted source or replacing the executor. Assert no wrong-source dispatch, stale delivery or false success and verify newly rendered availability; do not require messaging on detached controls.
- Delay generation and refresh a disposable preview without changing the accepted source; verify an authorized export can still finish. Change the accepted source/executor during work and assert that stale output does not deliver. Report browser download failures distinctly.
- Supplement mounted evidence with focused native Runtime/Session/Core and provider tests for exact generation and ownership. Extend existing Export row, Runtime lease/delivery, authored-case and CAD output tests. Run changed browser and native targets and record commands/results. Good tests observe artifact, blocker and ownership rather than private predicate shape.

## Out of Scope

- ZMK (ticket 001), KiCad/outline/footprint admission (ticket 002), portable project export and Export page styling.
- Replacing exact generation with previews, weakening leases or final delivery, adding missing CAD providers or guaranteeing fabrication readiness.
- Blocking saves for incomplete split projects or rewriting geometry algorithms as part of admission work.

## Further Notes

- [Candidate 02 research](../../../research/2026-10-20-export-readiness-dispatch-deepening.md) distinguishes output-specific admission from export ownership. [Generated exact geometry](../../../../GLOSSARY.md#language), [vendor model preview](../../../../GLOSSARY.md#language) and [export availability](../../../../GLOSSARY.md#mechanical-relationships) have different meanings.
- Preserve [ADR-0015](../../../adr/0015-exact-generated-cad-and-mesh-preview-scope.md), [ADR-0016](../../../adr/0016-export-ownership-is-separate-from-preview-currency.md), [ADR-0014](../../../adr/0014-document-authority-and-captured-lifetimes.md), [ADR-0021](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md) and [ADR-0027](../../../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md).
