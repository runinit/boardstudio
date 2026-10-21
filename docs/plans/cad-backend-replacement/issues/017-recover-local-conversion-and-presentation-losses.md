# 017: Preserve previews across local conversion and presentation losses

Status: ready-for-agent
Type: build
Blocked by: [016: Show surviving mesh geometry with errors](016-show-partial-meshes-with-errors.md)
Parent: [007: Vendor STEP import](007-strict-step-import.md)
Related: [018: Approximate collision feedback](018-show-collision-observations-for-incomplete-previews.md)
Spec: [Forgiving vendor previews](../spec.md)
Decision: [ADR-0017](../../../adr/0017-forgiving-vendor-previews-with-preserved-source.md)

## What to build

Extend the useful-preview path to recoverable source-conversion, occurrence and
presentation-item failures. Keep usable geometry visible with explicit omissions,
and retain the original vendor asset unchanged through save, reload and portable
archive round-trips.

## Acceptance criteria

- [ ] A genuine mixed valid/invalid face-conversion RED becomes GREEN through the real provider-to-page path: usable geometry and an explicit incomplete-model error coexist.
- [ ] A valid solid with unsupported or unretained presentation geometry remains visible and reports the omitted item. Recover where safe using existing loss reports, not a parallel parser or general repair framework.
- [ ] Failed occurrences are attributed and omitted only where safely identifiable; surviving occurrences preserve their transforms and unit conversion once. Unknown or invalid placement is never guessed.
- [ ] Retain source/face/occurrence identities where available and explicitly report unavailable provenance without inventing source references or treating omission as complete coverage.
- [ ] All-geometry failure is terminal and visible, while the same worker can subsequently import a valid asset. Unsafe parse/resource failures remain failures rather than manufactured partial models.
- [ ] Original bytes remain identical after attachment, saving/reload and portable archive export/import. Reconstructed or cached previews retain the same explicit limitations rather than silently becoming complete.
- [ ] Partial recovery never establishes whole-model clearance. Preserve incomplete-source admission metadata and stale/cancelled result protections established by 016.
- [ ] Contract obsolete report/capture machinery in the changed recovery routes only where no retained behavior, source ownership, diagnostics, known-budget enforcement or supported collision evidence depends on it. Migrate callers/tests and owner registrations in the same green slice; do not complete superseded certification packages.
- [ ] Record native behavioral RED/GREEN and affected real packaged-provider, worker, mounted/page and archive-round-trip checks. Preserve complete fixtures and malformed/oversized/invalid-buffer negative controls; no changed caps or repair allowances.
- [ ] Read the diff and obtain the applicable CAD contract/geometry review. Preserve the existing parent's claim and coordinate shared source integration with 018.

## Testing decisions

Use the spec's existing provider/worker-to-Runtime and mounted/page seams, with the
native conversion/import fixture matrix underneath. Reuse existing portable-archive
original-byte controls. Browser coverage verifies the new failure classes rather
than duplicating every native fixture or inventing test settlement logic.

## Scope boundaries

Not blocked by 018: partial-preview admission and original preservation do not require
new collision observations. General malformed-input salvage, new line-rendering
infrastructure, cap changes and exact vendor manufacturing are out of scope.
