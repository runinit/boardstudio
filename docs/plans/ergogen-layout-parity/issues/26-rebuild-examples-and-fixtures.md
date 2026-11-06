# 26: Rebuild the three matrix-owned examples and finish affected fixture updates

Status: resolved
Type: build
Blocked by: [24](24-relations-and-highlights.md), [25](25-project-format-recovery.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns only the example/starter rebuilds from ticket 14 and the golden updates those rebuilds directly cause; cutover fixture regeneration and gates are left to [17e](17e-fixtures-and-final-gate.md) and the end-to-end checklist to [27](27-end-to-end-acceptance.md).

## Outcome

Rebuild Split 42, Studio 60 and Macro 12 using the exact authoring recipes in decision
14, and audit all affected positive schema/scene/golden inputs after integration.
Preserve intentional legacy refusal fixtures. No new example collection or hardware.

## First RED / GREEN slice

Fail a real library-generated Studio 60 document asserting v7, a 5×12 matrix, one
Layout, only Row 2 Shift X=4.7625, existing poses and fresh-copy isolation. Build it
through Core rather than baking switch poses, then add Macro 12 as the next case.

## Subsequent acceptance

- Keep the same entries/identities, synthetic Demo switch, geometry-only disclaimer,
  19.05 mm pitch, 1.05 mm gap and 18×18 caps. New project remains blank. Saved user
  copies remain untouched; new members use proper matrix-coordinate identities.
- Studio/Macro origins and dimensions preserve 60/12 poses and zero headings exactly.
- Split has two same-board linked Layouts, four matrices and 42 keys. Use [decision 14](14-starters-and-demos.md#answer)'s
  left Main/Thumb mirror flags, row shifts, column staggers and relative anchor origin.
  Keep left/thumb poses; change only right-main row X by 0/−5/−10 mm as approved.
- Core maintains reflected counterparts and actual anchor movement, including edits
  from either half, preview/cancel and one Undo/Redo. No second right-side factory math.
  Ownership note: reflection is owned by [20](20-reflect-linked-layouts.md); verify it here only through the rebuilt examples.
- Preserve board/material/outline intent and update member/outline references. Add no
  controllers, wiring, physical pair or readiness claim; fresh inputs stay unassigned.
- Finish deliberate boundary/current-schema/outline/layout golden updates. Preserve
  companion, constraint, identity, changed-ID and persistence assertions; do not weaken
  them to bless new poses. Keep stress cases in focused tests and unrelated provider
  goldens unchanged unless a demonstrated dependency requires otherwise.
  Ownership note: cutover fixture regeneration is owned by [17e](17e-fixtures-and-final-gate.md); this ticket owns only goldens its example rebuilds directly cause.
- Keep fixture provenance/licences and the source builder as authority; no committed
  demo archive dump or runtime migration. Normal tests do not invoke Ergogen/Node.

## Checks

Sources: `web/crates/library/src/demo_projects.rs`, library/Runtime copy flow, boundary
fixture and Core integration tests. Use real factory documents + Core/Session acceptance,
mounted library/layout tests and affected quick/browser checks. Numeric planning
recipes alone are not production evidence. Regenerate packaged assets before page tests.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` at `846d6afef`. All slices
landed: `71b92fe41` (RED Studio 60 Core-generated matrix document), `b022a8f92`
(GREEN Studio 60 through Core matrix materialization), `1e3afe6db` (RED Macro 12
single-matrix recipe and shared example intent), `6a26189d7` (GREEN Macro 12
through the shared single-matrix builder), `dde2021e4` (RED Split 42 linked
layouts and approved pose deltas), `12d8c6496` (GREEN Split 42 with Core-derived
right half), `e07831e78` (Split reflection, one-Undo and preview-cancel through
Core), `59fc10764` (shared board/outline intent extended to Split 42), plus
rustfmt `846d6afef`. Both independent Spec and Standards reviews PASSED
(standards nits only, left as reviewed per integration scope). Native baseline
`818adfd32` (+11 names, 0 removals). Fresh gates green — quick: `check: all
steps passed: repo, tooling, lint, typecheck, test, browser`; full: `check: all
steps passed: repo, tooling, lint, build, test, browser`.

No release approval is implied.
