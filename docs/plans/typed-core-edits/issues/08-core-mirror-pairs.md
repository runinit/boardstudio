# 08: Core mechanical mirror pairs and linked features

Status: resolved
Type: build
Blocked by: 07
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [02](02-mechanical-settings-patch.md)

## Question / goal

Core resolves explicit `MechanicalMirrorPair`s: bidirectional linked features for
both a reused PCB and distinct left/right PCB designs, per-feature `Local`/`Omitted`
exceptions, Make local, Remove on this half, Rejoin and linked/local Delete. Pair
effects resolve before clearance projection, with one accepted edit and one Undo step
for the complete result. Pair and feature identities and exceptions survive
save/reopen. Fit conflicts are retained as located findings that block only affected
manufacturing output.

## Seams under test

- `CoreEngine` `Edit`/`Undo`/`Open` with pair and feature changes.
- `boardstudio_core::mechanical::edit` pair resolution and findings.
- Archive round-trip and `evaluate_edit`.

## TDD cases

1. Editing a linked mount/opening/support from either half reflects the counterpart
   with correct forward/inverse maps and reversed winding; a second reflection cannot
   hide in asymmetric geometry.
2. Same-PCB opposite-flip copies use the physical reflection; converted PCB drill
   coordinates are correct for every flip combination.
3. Distinct boards keep separate generated parts, references and memberships; one
   reused PCB unions required holes by board id and rounded XY.
4. Make local materializes current geometry without moving it; later counterpart
   edits leave it and the remaining links intact.
5. Linked Delete removes both; Remove on this half stores an omission; local Delete
   affects one half; Rejoin selects source geometry and clears only that exception.
6. A geometrically valid linked clash is accepted with a located fit finding and
   blocks only its affected assembly/board export.
7. Identities and exceptions round-trip through save/reopen and Undo/Redo.

## Verification

`cargo test -p boardstudio_core --locked`; WASM `typecheck`.

## Outcome

Resolved 2026-10-09 at integrated `2a8d5e076`. `MechanicalMirrorPair` stores one shared
construction and linked-feature records; `MechanicalFeatureCorrespondence` carries a
per-feature `source_instance_id` so each reviewed match chooses its own source half
(including a rejoin from the selected half's own geometry). Link, Unlink, Make local,
Omit and Rejoin are typed changes with explicit correspondences; linked additions join
the pair by default; `RemovePhysicalInstance` materializes the survivor and reconciles
replaced topology; the Q14 paired `Disable` materializes both halves, clears the pair and
disables only the selected half in one Undo step; `MaterializeLegacyConstruction`
converts selected Legacy members while preserving a third Legacy member's baseline.
Opening/vertex identities are reserved on `PhysicalBoardInstance` and survive
disable/enable and removal; pair geometry uses the reversible explicit `Pose2` map with
reflected winding and preserved vertex ids. Key commits: `b47e1ce00`, `4bde2993f`,
`86890fbcc`, `8aa0423ba`, `c89f9afc6`, `232f2d94a`, `155b7e01b`, `d8b7d8406`, `164634053`,
`de48fff72`, `56e3dd07b`, `c3f2f1494`, `619081368`, `42fbe88fc`.
