# 018: Show useful collision observations for incomplete previews

Status: ready-for-agent
Type: build
Blocked by: [016: Show surviving mesh geometry with errors](016-show-partial-meshes-with-errors.md)
Parent: [007: Vendor STEP import](007-strict-step-import.md)
Related: [014: Collision validation](014-vendor-mesh-collision-validation.md), [017: Conversion and presentation recovery](017-recover-local-conversion-and-presentation-losses.md)
Spec: [Forgiving vendor previews](../spec.md)
Decision: [ADR-0018](../../../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md)

## What to build

Let designers run collision checks on repaired or incomplete previews and see useful
approximate gaps or potential interference with explicit geometry/accuracy
limitations. A useful mesh observation is not a whole-model clearance guarantee.

## Acceptance criteria

- [ ] A genuine public-boundary RED for incomplete vendor geometry becomes GREEN with approximate gap or potential-interference observations shown alongside incompleteness and uncertainty.
- [ ] Cover vendor/vendor and vendor/generated operands using partial geometry already delivered by 016; do not wait for 017's additional conversion recovery routes.
- [ ] Preserve operand, witness and available source/occurrence identities. Missing source provenance is explicit, never fabricated; malformed transport indices and non-finite/degenerate query inputs retain safety handling.
- [ ] Missing geometry cannot yield a whole-model clear verdict or complete-source qualification. Unsupported requested accuracy stays uncertain; mesh refinement alone does not prove source accuracy or general containment.
- [ ] Complete-model supported accuracy classes and existing qualified conclusions remain working. Do not force every result to unknown merely because forgiving admission exists.
- [ ] Ordinary approximate checks do not require universal repair certification or automatic tighter refinement. Preserve on-demand refinement and distinguish requested accuracy from the physical clearance threshold.
- [ ] Mounted controls show observations and limitations for the model actually checked. Source changes, cancellation or owner replacement retire geometry-associated findings, including late partial completions.
- [ ] Record focused native/Runtime RED/GREEN, actual packaged-provider/worker checks and targeted mounted/page evidence. Reuse existing collision observation, uncertainty, witness and stale-result controls; do not add a parallel query/certification framework.
- [ ] Read the diff and obtain the applicable CAD boundary review; do not change exact generated manufacturing, numeric caps or existing allowances.

## Testing decisions

Reuse the Runtime collision-worker/lifetime tests, mounted Case uncertainty/witness
tests and existing real-page collision scenario, with actual provider checks for the
changed payload. No full browser duplication of the native fixture matrix is needed.

## Scope boundaries

Not blocked by 017. If 016 proves existing observation delivery already satisfies
these behaviors with only minimal propagation, propose an explicit scope merge with
the owner rather than inventing extra work or silently changing ticket status.
