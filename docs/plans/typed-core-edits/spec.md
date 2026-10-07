# Spec: Typed Core edits

Source: [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md) ·
Evidence: [whole-document resolver inventory](../edit-settlement/replace-document-resolvers.md)

## Problem

After [edit settlement](../edit-settlement/map.md), every product edit resolves against
the accepted document when it runs. About two dozen resolvers still answer with
`ReplaceDocument`. Each copies a domain rule into the web layer, makes Core report every
part and top-level outline feature as changed, and triggers script reapplication and
uncached outline recomputation, even for a single scalar. A resolver can also return a
document it captured outside its snapshot argument, which no type prevents.

## Goal

Product edits state what they change as typed Core operations. Core owns the rule,
reports the IDs it actually changed, and runs outline work only when the edit affects
outlines. `ReplaceDocument` remains for import and recovery, plus any bulk action a
later decision explicitly permits.

## Constraints

- ADR-0005 still holds: each typed edit is submitted as intent through `ResolveEdit`
  and an edit ticket, resolved against the accepted document at execution; vanished
  targets retire; equal values resolve Unchanged.
- One typed edit is one Undo step, exactly as the replacement was. Typed edits do not
  shrink Undo: Core history stores a full prior document per step.
- Operations are added to Core's `EditOperation`; serialized documents are not
  affected, but every operation needs Core tests at the Session/Core boundary.
- The strict paths (captured gesture commit, export commits, protected electrical
  remap) are unchanged.

## Out of scope

- Enforcing the import-and-recovery restriction (needs every bulk action classified).
- Finer-grained or smaller Undo history.
