# 22: Adapt setup, alignment, selection and geometry consumers

Status: claimed
Type: build
Blocked by: [21](21-exception-lifecycle.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns only the consumer wiring (Setup, Align, selection) that calls the Core anchor-align edit; the Core edit itself is left to [19](19-anchor-layout-groups.md).

## Outcome

Finish semantic consumer adaptation beyond the mechanical field cutover: one layout
kernel supplies positions/frames, pitch supplies defaults/size/snap, and several
matrices do not create a false shared transform context.

## First RED / GREEN slice

Fail a real Column Align on a bent column that requires rigid translation of the
selected targets. Route it through Core's exact column edit instead of client column
offsets; then add an anchored/Follow refusal case with unchanged document/history.

## Subsequent acceptance

- Setup/resize adds default layout records; pitch changes only unpinned steps. Keep
  snap units at selected pitch × snap fraction and size-based split gap calculations.
- Matrix Align uses the actual frame, Column Align uses the shared exact Core edit,
  Key/Component Align retain their own pose routes and Row Align stays unavailable.
  Exact operations must not silently break Local pose ownership or drop directions.
- Drafts, pair previews, keycap fit/size, plate openings and actual canvas cells use
  resolved centres but pitch-minus-gap sizes; nonuniform steps never resize keycaps.
- Adapt tree/Objects/context/selection for all Layout matrices and anchors. Keep flat
  evaluation-order listing and Follows detail; multi-matrix selections get frame-free
  actions only and no fabricated common parameter values/axes.
- Verify outlines/collision, CAD/PCB geometry, companions, electrical scan/pin ordering
  and keymap identity consume final member poses/identities, not a recreated grid.
  Do not refactor unrelated providers or normalize hardware data.

## Checks

Current sources include `web/crates/catalogue/src/matrix_setup_operation.rs`,
`web/crates/layout/src/objects/layout_align_controller.rs`, `web/crates/layout/src/part_placement.rs`,
UI-model tree/selection and keycaps/pair geometry consumers. Run focused native
consumer/Core contracts and `check.py quick --files <changed-paths>`, plus affected
browser subsets (`layout keycaps parts catalogue ui-model` as applicable). Report
subset evidence as partial; include native/export invariants actually affected.
