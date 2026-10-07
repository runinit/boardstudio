# 04: Resolvers state only their intent

Status: ready-for-agent
Type: build
Blocked by: 06, [edit settlement 20](../../edit-settlement/issues/20-preview-only-direct-edit-event.md), [typed Core edits 01](../../typed-core-edits/issues/01-set-wiring-mode.md) (its resolver sweep touches their files)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [01 answer](01-decide-pending-edit-settlement.md#answer) (item 5)

## What to build

Resolvers build a full `EditCommand` (`core/src/model.rs:1122`) although Session owns
three of its fields: it rejects a preview phase, sets `base_revision` from the snapshot
it resolved against, and fills an empty `transaction_id` (`application/src/session.rs:~1495-1510`,
at `915d305c0`). Web code writes `base_revision: 0` 31 times and
`transaction_id: String::new()` 47 times. Resolvers also check
`accepted.session_epoch != target.session_epoch` and retire with "The project is no
longer open." in 18 places, though Session rejects a stale epoch before any resolver
runs (`session.rs:~1424`).

Add `Resolution` constructors in `application/src/session.rs` that take only what the
resolver knows: target IDs and the operation, with an optional transaction ID for
resolvers that group edits (outline actions, layout groupings). Then:

- move every resolver to the constructors;
- delete the resolver-level session-epoch checks;
- keep resolver checks for a vanished or ineligible target (ADR-0005).

The `application/` change lands in its own small commit first. Edit settlement 20 also
changes Session's event types, so this follows it.

## Acceptance criteria

- [ ] No resolver in `web/` writes `base_revision`, `phase` or an empty
  `transaction_id` (`rg -n "base_revision: 0|transaction_id: String::new\(\)" web`
  finds only previews and strict captured routes).
- [ ] No resolver checks the session epoch (`rg -n "The project is no longer open" web`
  is empty, or each hit is justified in `## Outcome`).
- [ ] An Application test shows a resolved edit queued across a session change settles
  `Rejected(DOCUMENT_SESSION_CHANGED)` without calling its resolver.
- [ ] Existing tests pass; previews, gesture commits, export commits and the electrical
  remap keep their routes.

## Verification

```sh
cargo test -p boardstudio-application --locked
python3 scripts/check.py lint typecheck test browser
```
