# 18: Should `Event::Edit` be restricted to field-scoped operations at the type level?

Status: resolved
Type: grilling
Blocked by: 17
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## Question

After the migration every product commit is an intent, and `Event::Edit` is left
for previews and anything ticket 17 recorded. Its base-revision refresh is only
safe for payloads that are valid against any revision. Should the type system
enforce that, so a new panel can't reintroduce the overwrite?

Options to grill, using ticket 17's evidence:

- Keep `Event::Edit` as is and rely on review and the edit ticket being the easy path.
- Narrow `Event::Edit` to previews only; commits must be intents.
- Split the operation type into field-scoped and whole-document operations, and
  let `Event::Edit` accept only field-scoped ones.
- Remove the base-revision refresh, making `Event::Edit` strict.

For each, say what it costs (call sites, Core types, tests) and what it protects.

## How to work it

Call the `grilling` and `domain-modeling` skills. Resolve with `## Answer`; record an
ADR if the result is hard to reverse, otherwise a line on the map.



## Answer

Decided with the user on 2026-10-07 (grilling session), recorded as the
[ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-commits-are-intents-the-direct-edit-event-carries-previews-only-2026-10-07).

- **Scope:** guard the direct path only. Stale whole-document payloads returned by a
  resolver are not this ticket's problem; narrowing resolver outputs is the direction
  in [ADR-0006](../../../adr/0006-replace-document-for-import-and-recovery.md).
- **Level:** type-level. `Event::Edit` is replaced by a preview-only event that has no
  phase field; Session builds the preview command. A direct commit cannot be written.
  (Rejected: documented convention; runtime rejection of Commit while the type still
  allows it; removing the base-revision refresh, unnecessary once the event carries
  previews only.)
- **Tests:** no test-only escape hatch. All direct commits in tests (26 sites in 14 files
  on `a3b41ae8a`, including the shared `replace_document` helper) move to `ResolveEdit`
  through a helper that wraps a fixed command in a resolver.
  `queued_discrete_edits_use_each_preceding_durable_revision` is rewritten for queued
  resolvers.
- **Unchanged:** Session's captured gesture commit, export commits and protected
  electrical remap review stay strict and separate.
- **Implementation:** [ticket 20](20-preview-only-direct-edit-event.md) in this effort,
  done before the [typed Core edits](../../typed-core-edits/map.md) effort starts.

## Comments

2026-10-07: Prepared the [four-option investigation](../../../investigations/edit-event-restriction-options.md) and [decision brief](../decision-brief.md). They distinguish direct-event enforcement from unrestricted resolver outputs and preserve the strict-path constraints. The human decision and ticket-17 prerequisite remain open.
