# 19: Which remaining whole-document resolvers become typed Core edits first?

Status: resolved
Type: grilling
Blocked by: 17
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Source: [architecture review, candidate 2](../../../investigations/architecture-review-2026-10-06.html)

## Question

The migration makes whole-document replacements correct by building them from the
accepted document at execution. They still copy domain rules into web, report all part IDs and top-level outline
feature IDs as changed, and trigger outline recomputation. Architecture review candidate 2 proposes
intent-level Core edits (`RenameBoard`, `SetWiringMode`, `AddOutlineFeature`,
`SetMechanicalSettings { patch }`, `RenameDefinition` …) and restricting
`ReplaceDocument` to import and recovery.

Using ticket 17's `replace-document-resolvers.md`, decide:

- which concepts to convert first (ranked by user frequency, duplicated domain rules
  in web, and Undo/changed-ID cost);
- whether the first slice is a new effort with its own map;
- whether `ReplaceDocument` should eventually be restricted to import and recovery.

## How to work it

Call the `grilling` and `domain-modeling` skills. Resolve with `## Answer`; if a new
effort is chosen, its map is the destination of that effort, not this one.



## Answer

Decided with the user on 2026-10-07 (grilling session), recorded as
[ADR-0006](../../../adr/0006-replace-document-for-import-and-recovery.md).

- **Priority for the first slice:** the smallest bounded demonstration, so the pattern
  (operation shape, precise changed IDs, outline-affecting classification, Session/Core
  contract tests, resolver migration) is settled before larger concepts copy it.
  Frequency-first and domain-rule-first were considered; both are larger first designs
  and usage frequency is not measured.
- **First concept:** `SetWiringMode { board_id, mode }`, replacing the PCB wiring-mode
  resolver. Project and board renames are the likely next ticket.
- **New effort:** yes, [typed Core edits](../../typed-core-edits/map.md). Its map
  specifies only the `SetWiringMode` ticket; the other inventory groups are unspecified
  frontier. It starts after [ticket 20](20-preview-only-direct-edit-event.md).
- **`ReplaceDocument`:** reserved for import and recovery as the long-term direction,
  with no enforcement date. Archive open/import and recovery reopen, footprint import and
  routed-board reference upload are permitted by name; every other remaining
  replacement (physical setup, script apply, generator apply and model upload, assembly
  placement, reviewed wiring plan, matrix and mirrored-half creation, and the rest of the
  inventory) becomes a typed intent or gets its own decision to join the permitted list.

## Comments

2026-10-07: Prepared the [provisional resolver inventory](../replace-document-resolvers.md) and [decision brief](../decision-brief.md). Ranking is inferred, not measured usage; refresh after ticket 15/17. No concept, new effort or ReplaceDocument restriction has been chosen.
