# Draft spec addendum: Parts + Project visible parity

Status: draft for root planning/review only. Keep the parent graph at 62; these are child refinements and evidence notes, not new parent tasks. See `REPORT.md` and `ACTION-TRAIL.md` for paired browser evidence.

## Shared acceptance invariants

- Public paired browser UI is the acceptance surface; use the same fixture, title, action, theme and viewport in React and Dioxus.
- A Parts library query/selection/preview is read-only with respect to the single accepted project/session. Parts 2D/3D sample geometry comes from an isolated ephemeral sample document; that sample is never a second authoritative project/session.
- Query, scope, async source loads, preview compilation and renderer callbacks retain captured identity. Stale results after project/selection changes are ignored with visible current errors; they cannot overwrite a newer selection or accepted draft.
- Browse/panel/view changes do not increment document revision, add history, alter assembly selection, or use Session part selection to represent a 3D layer selection.
- No public API/model/CAD/schema/catalogue changes or invented preview rows. Use existing Ergogen, imported static definitions, current project overrides, module catalogue, footprint projector, and renderer exports with explicit source/provenance.
- Project entry and Parts panel slots remain shared-shell/coordinator-owned. Feature slices own only new private presentation/controller logic within a reviewed seam; root owns shared mount and CSS.

## Initial focus (not a waiver of later acceptance)

- The first Projects demonstration may show two existing keyboard fixtures: Sofle v2 and REVIUNG41. These are the requested initial choices. Keep final/full project-gallery acceptance visible in parent F2.1; two choices are not evidence for complete demo parity.
- The first Parts preview/control demonstration should be the actual selected component’s 2D footprint with named layer toggles, supported by the existing footprint projection path. 3D camera/model/render controls are a distinct shared viewer consumer and keep F7.3 + INT.2 as F4.4 full-acceptance joins.
- The project-start slice should expose New/Create and open the existing real setup guide for a blank project. New, saved-open, import, and portable-copy behavior must preserve one current Session/project identity. The deeper durable archive/open/delete race/error acceptance stays F2.2 + INT.2.

## Ticket-boundary reconciliation

- Existing Parts child 01 remains component catalogue/detail only; do not add assembly authoring, placement, generator edits or previews to it.
- Existing Parts child 02 remains bundled key-assembly presets + source-backed module variants, starting after child 01.
- New child draft `04-first-2d-footprint-preview-controls.md` refines the first visible part-preview path with current footprint projection; it does not claim completion of F4.4’s 3D viewer acceptance.
- Existing Parts child 03/F4.4 remains the isolated full library preview ticket. It starts after F4.1 + F7.1 and joins F7.3 + INT.2 at acceptance.
- New Project child draft `05-project-start-create-two-demos.md` refines the first visible create/demo route under F2.1. Root owns mount/lifecycle wiring and the feature author owns only its private leaf. F2.2 remains the lifecycle/archive/open/delete acceptance ticket after F2.1, with INT.2 acceptance join.
