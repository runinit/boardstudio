# 08: Core mechanical mirror pairs and linked features

Status: ready-for-agent
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
