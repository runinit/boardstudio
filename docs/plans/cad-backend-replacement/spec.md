# Forgiving vendor previews and honest collision feedback

Status: ready-for-agent
Type: spec
Decision: [ADR-0017](../../adr/0017-forgiving-vendor-previews-with-preserved-source.md), [ADR-0018](../../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md)
Related: [CAD map](map.md), [007](issues/007-strict-step-import.md), [014](issues/014-vendor-mesh-collision-validation.md)

## Problem Statement

A vendor model with useful geometry can be refused wholesale because one face cannot
be converted or meshed. Older CAD instructions also require completeness and extensive
repair certification before admitting a preview. That prevents useful display and
encourages disproportionate implementation work. Conversely, simply hiding import
errors would leave users unaware of missing geometry or unsupported collision accuracy.

## Solution

Show the safely usable geometry, with an explicit incomplete-model error when any
presented geometry is missing. Show uncertified repairs with warnings rather than
requiring a mathematical repair certificate merely to display them. Preserve the
untouched original independently of the preview. Allow useful approximate collision
observations, while keeping unsupported accuracy and whole-model clearance uncertain.
If no usable geometry survives, terminate with a visible failure rather than an empty
successful preview. Input and resource safety remain mandatory.

## User Stories

1. As a designer, I want ordinary complete vendor models to keep displaying, so that forgiving import does not regress working assets.
2. As a designer, I want a repaired preview even when its error cannot be certified, so that I can position useful vendor models.
3. As a designer, I want repair limitations shown as warnings, so that display success is not mistaken for exactness.
4. As a designer, I want surviving faces shown when another face cannot be meshed, so that one failure does not hide the whole model.
5. As a designer, I want surviving faces shown when local source conversion fails, so that recoverable conversion losses do not abort useful display.
6. As a designer, I want an explicit error beside incomplete geometry, so that omissions cannot silently mislead me.
7. As a designer, I want omissions attributed to their source face or occurrence where available, so that I can understand which geometry is missing.
8. As a designer, I want unavailable provenance identified honestly, so that guessed source identities do not appear authoritative.
9. As a designer, I want usable solids retained when a presentation item is unsupported, so that an unrelated curve does not suppress the solid preview.
10. As a designer, I want repeated occurrences to retain their correct placements, so that partial recovery does not collapse or double-transform assemblies.
11. As a designer, I want unit conversion preserved, so that a forgiving preview still has the correct physical scale.
12. As a designer, I want invalid placement to be diagnosed rather than guessed, so that recovery does not manufacture a misleading assembly.
13. As a designer, I want a visible terminal error when nothing usable survives, so that import does not report empty success or remain pending forever.
14. As a designer, I want a valid import to work after a failed import, so that one bad model does not poison the worker.
15. As a designer, I want original vendor bytes retained unchanged, so that preview repair cannot destroy my source asset.
16. As a designer, I want reload and portable archive round-trips to preserve those bytes and limitations, so that saving does not turn a partial preview into an apparently complete model.
17. As a designer, I want approximate gaps and potential interference reported for repaired or incomplete previews, so that imperfect models still provide useful feedback.
18. As a designer, I want collision findings to retain operand and available source identity, so that I can locate the geometry actually checked.
19. As a designer, I want incomplete models prevented from establishing whole-model clearance, so that absent geometry cannot create a false clear verdict.
20. As a designer, I want unsupported requested accuracy to remain uncertain, so that a mesh measurement is not presented as a physical guarantee.
21. As a designer, I want supported complete-model conclusions preserved, so that forgiving admission does not discard existing useful evidence.
22. As a designer, I want ordinary checks to avoid mandatory universal proof or refinement, so that useful feedback does not depend on completing a certification project.
23. As a designer, I want cancelled or stale requests to publish neither geometry nor errors into a replacement scene, so that diagnostics belong to the model being viewed.
24. As a designer, I want malformed, oversized or unsafe input rejected predictably, so that forgiving geometry handling does not weaken application safety.
25. As a designer, I want exact generated case and PCB output unchanged, so that preview recovery cannot silently replace manufacturing geometry with approximations.
26. As an implementing agent, I want one current admission policy and clear successor work, so that historical strict-import plans do not override accepted decisions.

## Implementation Decisions

- Preview usability, completeness, source identity and supported collision accuracy are separate facts. Unknown geometric error alone does not make an otherwise useful preview unusable.
- Recover locally from conversion, meshing and unsupported-presentation losses where safe. Never invent missing geometry, units, placements or source provenance. An omission must produce an explicit diagnostic.
- Geometry and its bounded diagnostic/completeness metadata travel together through the existing provider, worker, cache, Runtime and presentation boundaries. Cached or reused geometry must not lose its incomplete status.
- Complete uncertified repairs receive limitations; incomplete previews receive an error while surviving geometry remains visible. The exact visual layout is not prescribed, but diagnostics must be accessible and associated with the affected model.
- If no usable surface or supported line geometry survives, return a terminal visible failure. This is not permission to fabricate an empty-success model or add a new general line renderer.
- Keep original vendor bytes independently of disposable previews. Reload and project archive handling must not replace originals with repairs.
- Approximate observations do not require whole-source qualification. Supported clearance conclusions still require adequate evidence; incomplete models cannot qualify as complete merely because their surviving geometry resembles a supported class.
- Retain current malformed-input, resource, finite-coordinate, buffer/index, source-identity, units, placement, cancellation and stale-result safeguards. Preserve existing known repair-budget enforcement; do not introduce new caps, tolerance floors or repair allowances.
- Reuse existing conversion-loss reporting and collision/lifetime mechanisms. Remove proof-only capture, retention or bookkeeping only where it has no remaining role in source ownership, diagnostics, supported conclusions or existing safety/budget guards. Do not complete obsolete certification packages as a prerequisite.
- Exact generated case/PCB manufacturing remains separate. Forgiving vendor previews do not confer exact vendor manufacturing authority or general containment guarantees.
- The accepted ADRs and this spec govern successor work. Historical strict criteria remain traceable but cannot gate preview admission. Existing claimed work requires coordination, not claim takeover or silent cancellation.
- No persisted document-schema change or new public API is assumed in advance. Any necessary cross-boundary contract change must be carried end to end in the owning slice and reviewed under the existing CAD contract rules.

## Testing Decisions

The user approved the two acceptance surfaces and four-ticket breakdown on 2026-10-19.
Tests assert externally visible geometry, diagnostics and conclusions, not private
ledger shapes or implementation-specific helper counts.

1. **Actual provider/worker to Runtime.** Use existing native import regressions for the fixture matrix, actual packaged WEB provider checks for the changed boundary and selected real-worker integration controls. Cover mixed good/bad faces, conversion and presentation losses, transformed occurrences, all-failed input, same-worker recovery, preserved original bytes, safety refusals and stale/cancelled completions.
2. **Targeted mounted UI/page.** Show that surviving geometry is actually submitted/rendered while an accessible incomplete-model error is visible for the same model. Exercise real-page provider/worker delivery and collision observations; a formatter-only test, fake settlement or an error with no rendered geometry cannot establish acceptance.

Prior art includes the existing STEP import and face-completeness suites, packaged
surface-preview and collision/qualified-analysis checks, Runtime collision worker
and lifecycle tests, shared-viewer projection probes, mounted Case uncertainty and
source-change tests, the real-page collision scenario and portable archive round-trips.
Native workflow tests use the real Session/Core with controlled effects; browser
coverage retains behavior unavailable natively, as required by
[ADR-0021](../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md).

Each changed nontrivial route needs a genuine failing behavior regression before the
fix, then GREEN evidence through its public boundary. Do not merely invert every
strict test: distinguish recoverable local loss from unsafe input. Existing complete
asset behavior and supported collision classes are positive controls. Test-name
baselines and owner/provider registrations must remain intentional and nonempty.
Evidence is tied to the tested source; do not repeat unrelated corpus or benchmark
runs for unchanged code.

## Out of Scope

- Universal repair certification, general B-rep/STEP validation or a new proof framework.
- New parsers, generic repair/report frameworks or a new testing infrastructure.
- Raising caps, changing numeric repair budgets, or removing known-budget guards without a separate decision.
- General containment certification, exact vendor manufacturing, or generated CAD/keycap redesign.
- Silent loss of geometry, fabricated placements/provenance or unsupported clear verdicts.
- Editing or closing parent 007, reopening resolved migration tickets, discarding current owner work, or treating publication as implementation completion.
- Hardware-topology redesign, other ADR research and unrelated UI cleanup.

## Further Notes

### Successor task graph

| Ticket | Delivers | Blocked by |
| --- | --- | --- |
| [015](issues/015-reconcile-forgiving-preview-policy.md) | Reconciled active instructions and reference-contract admission policy | None |
| [016](issues/016-show-partial-meshes-with-errors.md) | Visible surviving meshed geometry with atomic incomplete-model diagnostics | 015 |
| [017](issues/017-recover-local-conversion-and-presentation-losses.md) | Partial recovery across conversion/occurrence/presentation losses and original-byte round-trips | 016 |
| [018](issues/018-show-collision-observations-for-incomplete-previews.md) | Approximate collision feedback for incomplete previews without false whole-model clearance | 016 |

Tickets 017 and 018 are behaviorally independent after 016. Shared-file ownership
still requires coordination; an integration conflict is not a hidden behavioral
dependency. If 016 establishes that existing collision observation delivery needs
only minimal propagation, the owner may propose folding 018 into it rather than
inventing a separate collision project; do not silently close or widen tickets.

The source parent [007](issues/007-strict-step-import.md) remains claimed, with its
historical criteria and current work untouched. This does not require completing its
superseded six-package certification track. The resolved [014](issues/014-vendor-mesh-collision-validation.md)
and [010](issues/010-production-cache-worker-browser.md) provide existing integration
evidence, not new certification prerequisites. The dated current-policy notice in
the [map](map.md#current-vendor-preview-policy-2026-10-19) takes precedence over their
older contradictory admission language.

Publication is planning only: no provider, worker, Runtime or UI behavior has been
changed or verified by this spec. Recheck concurrent source and claims before work.
