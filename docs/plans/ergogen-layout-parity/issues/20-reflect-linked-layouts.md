# 20: Reflect complete layout groups and inverse-edit their actual frames

Status: claimed
Type: build
Blocked by: [19](19-anchor-layout-groups.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns linked group reflection (actual/transported frames, both linking systems, inverse edits); the Local/Omitted resize, Rejoin, confirmed detach and source-removal lifecycle is left to [21](21-exception-lifecycle.md).

## Outcome

Support the new layout data and anchors through both same-board Layout links and
native mirrored halves, retaining their distinct ownership systems but one set of
geometry/exception meanings. No second mirror math in a web factory.

## First RED / GREEN slice

Fail a prepared mirror-originating Shift/Bend/Key rotation edit that reflects all
local heading exactly once under a translated/slanted map. Pass it using the shared
kernel and exact inverse; then add an anchored group counterpart case.

## Subsequent acceptance

- Copy new local records and cell turns unchanged. Toggle None/X; root Origin is a
  mapped point and Rotation `φ−r`; anchored Origin X/Rotation negate relatively.
  Do not use forward mapping as the point inverse or negate cell rotation twice.
- Map all Layout members/anchor targets by identity and physical key, independent of
  list order. Preserve numbering, companions, hardware overrides and unrelated links.
- Apply Follow after the candidate and transport its source-world basis. Test the
  non-origin-axis example, slanted maps and unfollowed rotation.
- A dependent follows its actual Local anchor while its own parameters stay linked;
  inverse edits use that actual frame. Reflect compensation once, including column 0.
- Complete same-board per-matrix/key Local/Omitted/Rejoin support consistently with
  native records; do not add per-column exceptions or allow double ownership.
  Ownership note: the Local/Omitted/Rejoin lifecycle is owned by [21](21-exception-lifecycle.md).
- Preserve native companion-mode/side/heading distinctions and existing linked Y,
  ambiguous-alias, simultaneous-target and constraint refusals.

## Checks

Current seams: `core/src/matrix/layout.rs`, `core/src/mirrored_half/{inverse,reconcile}.rs`
and prepared transaction flow. Use real Core/Session edits, preview/commit and Undo
from both halves in both systems. Run focused mirror/matrix/companion tests and quick
checks; the upstream harness alone is not acceptance for these extended rules.
