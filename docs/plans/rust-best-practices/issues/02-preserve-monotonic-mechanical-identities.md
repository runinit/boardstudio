# 02: Preserve monotonic mechanical feature identities across removal, undo, and overflow

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** Mechanical feature additions (`openings`, `vertices`, `criticalFits`, `hardware`, and `mounts`) mint monotonically increasing identities that are never re-issued after removal or after undoing an addition within one document session. A pending field edit targeting a removed or undone item must not mutate its replacement. Allocation and reservation merging cleanly reject exhausted numeric namespaces without partial document changes.

## Research decision (2026-10-19)

Rewrite, retain the bug fix: openings/vertices already preserve counters on ordinary removal, but Undo restores older counters; critical fits, hardware and mounts search for unused low ordinals and can reuse removed IDs. Non-strict pending edits resolve against the latest accepted document, so a base-revision check does not prevent redirection to a reused ID. A sparse imported `MAX` suffix does not currently overflow the first-gap allocators; respecting that suffix becomes part of the new monotonic allocation contract, not evidence of an existing panic in every allocator. See [research 02](../../../research/rust-best-practices-ticket-validation.md#02--monotonic-mechanical-identities).

Keep allocation authority in Core. Preserve high-water reservations outside Undo snapshots, or merge them on history restoration even when a configuration/owner is absent. Retain existing serialized counters, identity formats, canonical board/physical-instance ownership and mount collision exclusions; no UUID dependency or UI tombstone system. Seed reservations from imported IDs/counters. A successful explicit Open starts a new session; failed Open, preview/evaluation, failed edits, panel remount, Undo/Redo and disable/re-enable do not reset reservations. Recovery that reconstructs Core must either retain reservations or start a new session epoch so old pending edits retire. Ticket 01 is related atomicity work, not a prerequisite.

## Acceptance criteria

- [ ] For all five feature kinds, remove+add and add+undo+add assign fresh identities; setters targeting the old identity cannot change replacement values. Preserve existing opening/vertex removal protection and mount cross-collection collision checks.
- [ ] Application/Core integration tests use the real queued resolver for both first-gap reuse and history reuse, keeping the document session and target scope unchanged. An obsolete setter is retired or rejected, not redirected; do not substitute a raw old-base-revision command that fails before the resolver runs.
- [ ] Reservations remain owner-scoped and survive Undo/Redo, undo of configuration creation, and disable/re-enable; separate owners do not share an accidental counter. A failed Open preserves reservations and a successful explicit Open resets the session boundary.
- [ ] Imported live suffixes and stored reservation counters seed allocation. Tests cover sparse IDs, `MAX-1`, `MAX` counters and suffixes, and clean exhaustion for all five namespaces; do not saturate into reusable identities.
- [ ] Exhaustion during allocation or reservation capture/merging (disable, targeted replacement, pair dissolution/materialization) returns a clean error with unchanged accepted state. Native Core/application tests cover these transitions; browser tests cover affected queued-edit behavior.
