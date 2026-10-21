# 016: Show surviving mesh geometry with an incomplete-model error

Status: ready-for-agent
Type: build
Blocked by: [015: Reconcile current CAD policy](015-reconcile-forgiving-preview-policy.md)
Parent: [007: Vendor STEP import](007-strict-step-import.md)
Related: [010: Provider and browser integration](010-production-cache-worker-browser.md), [018: Approximate collision feedback](018-show-collision-observations-for-incomplete-previews.md)
Spec: [Forgiving vendor previews](../spec.md)
Decision: [ADR-0017](../../../adr/0017-forgiving-vendor-previews-with-preserved-source.md), [ADR-0018](../../../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md)

## What to build

When source geometry has converted successfully but one face cannot be meshed,
display the safely usable remaining geometry and an accessible incomplete-model
error for that same model. Carry the limitation with geometry through the actual
provider, worker, cache, Runtime and viewer; do not turn an error into silent success.

## Acceptance criteria

- [ ] A genuine RED using mixed usable/unmeshable converted faces becomes GREEN: surviving geometry is rendered while an explicit incomplete-model error is visible. Known omissions have available face/occurrence attribution.
- [ ] Geometry, bounded diagnostics and completeness remain associated through real packaged WEB provider delivery, typed worker transport, caching/reuse and mounted/page rendering. Cached geometry never sheds its limitation.
- [ ] Repeated transformed occurrences retain correct surviving geometry and omission attribution with units/placements applied once.
- [ ] Useful complete uncertified repairs remain previewable with honest warnings. Unknown repair error alone does not demand universal certification or refinement.
- [ ] Incomplete geometry cannot obtain whole-source qualification or a whole-model clear verdict immediately in this slice, even before collision-observation enhancements land. Keep supported complete-model behavior working.
- [ ] If no usable surface or supported line geometry remains, publish a visible terminal failure, not empty success or endless pending. A subsequent valid request succeeds on the same worker.
- [ ] Cancellation, source replacement and stale completions suppress both obsolete geometry and its diagnostics; the UI never combines different owners' results.
- [ ] Retain finite coordinates, valid indices, safe buffers, canonical source identity, invalid-option/resource refusals and existing known-budget guards. Do not weaken those checks to obtain a partial success.
- [ ] Remove proof-only capture/retention made demonstrably unnecessary in the changed meshing path only after establishing that it serves no retained diagnostic, provenance, supported accuracy or enforced safety/budget role. Preserve error handling and callers; add no new generic report framework.
- [ ] Record focused native RED/GREEN, actual packaged-provider and real-worker evidence, plus targeted mounted/page evidence of geometry and error visibility together. Retain positive complete-model and negative safety controls; update affected owner/test registrations and test-name baselines intentionally.
- [ ] Review public contract/geometry changes under existing CAD review rules. Coordinate overlapping claimed work; no unrelated cleanup or changed numeric policies.

## Testing decisions

Use the spec's two approved acceptance surfaces. Prior art is the existing STEP
face-completeness/surface-preview suites, Runtime model delivery/lifetime checks and
shared-viewer projection tests. A synthetic formatter assertion or manually settled
fake Runtime cannot substitute for actual geometry-plus-error delivery.

## Scope boundaries

This slice starts with successfully converted source geometry. Additional local
conversion/presentation recovery belongs to 017; new approximate collision delivery
belongs to 018. The anti-false-clear boundary is not deferred to either ticket.
