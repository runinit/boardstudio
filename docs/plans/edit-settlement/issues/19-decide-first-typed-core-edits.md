# 19: Which remaining whole-document resolvers become typed Core edits first?

Status: ready-for-human
Type: grilling
Blocked by: 17
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Source: [architecture review, candidate 2](../../../investigations/architecture-review-2026-10-06.html)

## Question

The migration makes whole-document replacements correct by building them from the
accepted document at execution. They still copy domain rules into web, mark every
ID changed and recompute every outline. Architecture review candidate 2 proposes
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
