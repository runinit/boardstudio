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
- Adding an operation to Core's `EditOperation` does not itself change serialized
  documents; every operation needs Core tests at the Session/Core seam. The
  [mechanical settings extension](issues/02-mechanical-settings-patch.md) also adds
  persisted mirror-pair and feature-exception semantics. Specify their representation
  and existing-project compatibility under
  [ADR-0004](../../adr/0004-document-format-version-and-id-rename.md) before building it.
- The strict paths (captured gesture commit, export commits, protected electrical
  remap) are unchanged.

## Approved outline extension

[Ticket 03's Answer](issues/03-decide-outline-version-intents.md#answer), confirmed
2026-10-09, includes linked outline refinements alongside the typed-operation migration.
Core owns one typed outline-edit interface, identities, atomic lifecycle and linked
resolution. Linked refinements follow current included components and keep independent
settings; source removal cleans only dependent choices and recalculates bridges. Freeze
or manual perimeter editing makes a fixed copy. Persisted ownership survives reopen;
existing fixed outlines do not gain links. Stale reviewed targets retire, malformed
input errors, and located geometric findings block only affected exports while keeping
saving/editing available. Implementation is sliced into tickets 11-15.

## Out of scope

- Enforcing the import-and-recovery restriction (needs every bulk action classified).
- Finer-grained or smaller Undo history.
