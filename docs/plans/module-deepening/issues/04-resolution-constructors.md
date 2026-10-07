# 04: Resolvers state only their intent

Status: resolved
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

- [x] No resolver in `web/` writes `base_revision`, `phase` or an empty
  `transaction_id` (`rg -n "base_revision: 0|transaction_id: String::new\(\)" web`
  finds only previews and strict captured routes).
- [x] No resolver checks the session epoch (`rg -n "The project is no longer open" web`
  is empty, or each hit is justified in `## Outcome`).
- [x] An Application test shows a resolved edit queued across a session change settles
  `Rejected(DOCUMENT_SESSION_CHANGED)` without calling its resolver.
- [x] Existing tests pass; previews, gesture commits, export commits and the electrical
  remap keep their routes.

## Verification

```sh
cargo test -p boardstudio-application --locked
python3 scripts/check.py lint typecheck test browser
```

## Outcome

Merged `deepening/04-resolution-constructors` through `9bfefe8dd`, rebased onto
`b3d44473`. The Application interface adds `Resolution::submit` and
`submit_with_transaction_id`; Session retains phase/revision/transaction ownership.
The web resolver sweep preserves explicit grouping IDs and target eligibility, removes
redundant resolver epoch checks, and includes the new native Runtime test helpers.
Remaining command-field literals belong to previews, strict captured routes or explicit
test setup; result pattern matches are not resolver construction. The project-no-longer-
open resolver message is gone. Application coverage proves stale queued resolvers are
rejected before invocation.

Final lint/typecheck passed. After the native Runtime integration, native Application
30, Runtime 100, Layout 52 and Parts 37 tests passed, as did the PCB package tests.
Targeted WASM Runtime 29, PCB 38 and Parts 50 passed. Earlier complete workspace and
footprints checks and the affected downstream browser suites passed; the broad gate
attempt recorded the known intermittent Keymap timeout and unchanged CAD fixture
baseline (49 passed, 1 failed, 4 ignored). Keymap subsequently passed in the complete
workspace-state browser gate. Final Standards and source-corrected Spec reviews found
no actionable findings; the final docs-only rebase changed no implementation files.
