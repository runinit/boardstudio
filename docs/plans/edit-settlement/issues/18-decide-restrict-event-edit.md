# 18: Should `Event::Edit` be restricted to field-scoped operations at the type level?

Status: ready-for-human
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


## Comments

2026-10-07: Prepared the [four-option investigation](../../../investigations/edit-event-restriction-options.md) and [decision brief](../decision-brief.md). They distinguish direct-event enforcement from unrestricted resolver outputs and preserve the strict-path constraints. The human decision and ticket-17 prerequisite remain open.
