# 015: Reconcile active CAD instructions with forgiving previews

Status: claimed
Type: task
Blocked by: None (can start immediately)
Parent: [007: Vendor STEP import](007-strict-step-import.md)
Related: [014: Collision validation](014-vendor-mesh-collision-validation.md)
Spec: [Forgiving vendor previews](../spec.md)
Decision: [ADR-0017](../../../adr/0017-forgiving-vendor-previews-with-preserved-source.md), [ADR-0018](../../../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md)

## What to build

Give an implementing agent one unambiguous current policy: useful repaired or partial
vendor previews with visible diagnostics, preserved originals and honest collision
uncertainty. Reconcile active reference instructions without erasing historical
evidence or taking over the existing importer owner's work.

## Acceptance criteria

- [x] The CAD map's current-policy notice, spec and successor task graph consistently override certification/completeness as preview-admission gates; dated migration evidence stays historical.
- [x] Reconcile the analysis boundary's completeness language: incomplete geometry may be displayed and queried approximately, but cannot establish whole-source qualification or clearance. Correct stale qualification-v2 pending/inactive descriptions against current delivered behavior, without fabricating new acceptance evidence.
- [x] The earlier STEP-validation-simplification map clearly points to the successor policy where its aggregate-completeness admission rule conflicts. Do not reopen its resolved/wontfix work.
- [x] Name the obsolete 007 owner-route/region-proof/package exits externally as non-prerequisites for preview admission. Preserve parent 007's body, claimed status, acceptance boxes, current work and evidence; do not edit or close it. Preserve resolved 010/014 bodies and outcomes as well.
- [x] Preserve input/resource safety, original bytes, units/placements, known-budget guards, supported complete-model collision conclusions and exact generated manufacturing scope. Do not reinterpret unknown accuracy as clear.
- [ ] Coordinate the successor source ownership with the claimed importer owner before overlapping implementation. No new requirement to complete its superseded packages is introduced.
- [ ] Read the documentation diff and run the existing documentation/link and repository consistency checks. Publication or document reconciliation is not provider implementation evidence.

## Verification

Use the existing repository documentation/link checks and a full diff read. No CAD
build, corpus rerun or source modification is needed for this documentation slice.

## Scope boundaries

Do not remove code or change behavior here. Later tickets own public-boundary RED/GREEN
evidence. Historical criteria remain retrievable, but are not active acceptance gates.

## Progress

Commits `5fe768750` and `e80da69dc` reconcile the active qualification wire and older STEP-validation documents, correct the shared fixture's stale status text, and distinguish accepted forgiving-preview policy from the still-undelivered 016–018 routes. The CAD map names the superseded 007 exits externally; 007, 010 and 014 bodies remain untouched. The documentation diff, JSON fixture parse and CAD-map link path were checked. `python3 scripts/check.py repo` in the isolated worktree failed only on inherited cleanup-map links to untracked files; the dirty root separately has pre-existing deleted-`CONTEXT.md` links. Coordinate any overlapping 016–018 source work with 007's owner, then close this ticket only after the current documentation set is integrated and repository checks pass. No CAD build or corpus rerun was performed for this documentation slice.
