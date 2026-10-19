# 02: Preserve monotonic mechanical feature identities across removal, undo, and overflow

Status: resolved
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

## Answer

Resolved on `t3/implement-rust-best-practices` (base `c071dedcf`).

- `CoreEngine` owns a session `MechanicalIdentityLedger` outside Undo snapshots, seeded from
  live IDs and stored counters by a successful explicit Open. It is not reset by failed Open,
  preview, failed edit, Undo/Redo, disable/re-enable or panel remount.
- The ledger floors are merged into mechanical allocation through
  `apply_with_ledger` / `prepare_transaction_with_ledger`. Opening, vertex, critical-fit,
  hardware and mount allocation is counter-based and checked; `u64::MAX` is the exhausted
  sentinel and is never saturated into a reusable identity. Owner scopes are board, physical
  instance, and per-opening vertex. Mount exhaustion still excludes Part and Case-body mount
  collisions, now restricted to the canonical `case-mechanical-mount-N` namespace.
- New counters are persisted on `MechanicalConfiguration` and
  `MechanicalIdentityReservation`; absent/zero means unset, so no format migration is needed.
  A successful Open starts a new session and reseeds from the received document. Reservation
  capture and pair merge return clean errors on exhausted floors; edits remain staged.
- Tests: `core/tests/mechanical_identity_monotonic.rs` (five kinds remove+add, add+undo+add,
  stale setters, sparse IDs, MAX-1/MAX counters and suffixes, owners, disable/re-enable,
  stored reservations, failed/successful Open, redo and undo of creation),
  `core/src/mechanical/edit.rs` merge unit tests,
  `core/tests/mechanical_integration.rs` pair-dissolution exhaustion,
  `application/tests/durable_session.rs` real queued resolvers for first-gap and history reuse
  (production `evaluate_edit` -> Submit -> Core rejection shape), and
  `web/crates/case/src/mechanical_settings_controller.rs` browser queued-setter coverage.

Evidence: Core lib 794 passed; application suite passed; case browser crate 82 passed; host
41 passed; `check.py test --receipt` 2057 native tests with zero ZipWriter diagnostics.
