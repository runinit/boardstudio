# 03: Session resolves pending edits against the accepted document

Status: ready-for-agent
Type: build
Blocked by: 02
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Add a way to submit a pending edit as intent rather than as a ready-made command.
Session holds the intent in its queue. When the edit reaches the head of the queue
and nothing else is running, Session hands the intent the accepted snapshot and
asks it for a command. The intent answers with one of:

- **Submit** a commit command. Session runs it against Core as usual, with the base
  revision taken from the snapshot.
- **Unchanged**. Session settles it completed, landing at the current accepted
  snapshot, with no Core request and no new revision.
- **Retire** with a reason. Session settles it `Rejected(reason)`.

With this in place, the investigation's X-then-Y sequence keeps both coordinates.
Undo removes Y, then X.

## Background you need

- Read [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md), the
  spec's "Implementation Decisions" and
  [the investigation](../../../investigations/layout-part-editing.md) (its
  constraints section especially).
- Session today:
  - `Event::Edit` is enqueued non-strict;
  - `pump` runs only when no Core request is active, no save is pending and Session
    isn't in recovery or closed;
  - for non-strict edits it overwrites `command.base_revision` with the accepted
    revision and leaves the payload untouched.
  Because `pump` only runs in that idle state, the accepted snapshot it sees is
  exactly the document Core will apply the next command to. That's what makes
  resolving at this point sound.
- Intents queued in an older session epoch are already rejected when popped.
  Recovery and closing already refuse new intents and block the queue. Keep both
  behaviours for the new event.
- Prior art for tests:
  - `application/tests/durable_session.rs` drives Session and a real `CoreEngine`
    by hand, with `core_effect`/`save_effect` helpers;
  - `docs/investigations/layout-part-editing-repro.rs` is the overwrite
    reproducer.
- Starting points (at `9548275`): `application/src/session.rs` (`Event`,
  `IntentKind`, `enqueue`, `pump`, `core_completed`).

## Shape (decision-rich sketch, adapt names)

```rust
pub enum Resolution {
    Submit(EditCommand), // phase must be Commit; base_revision is overwritten by Session
    Unchanged,           // settles Completed, landing = current accepted snapshot
    Retire(String),      // settles Rejected(reason)
}

/// Pure function of the accepted snapshot and the values the owner captured.
/// Must not read UI state, signals or Runtime.
#[derive(Clone)]
pub struct EditResolver(Rc<dyn Fn(&AcceptedSnapshot) -> Resolution>);
// Debug prints a label; PartialEq compares Rc pointers (Event derives both).

Event::ResolveEdit { operation_id: OperationId, label: String, resolver: EditResolver }
```

`label` is a short description for debugging and transaction IDs. Session derives
the transaction ID from the label and operation ID unless the command supplies one.

## Approach (test-first)

1. Turn the investigation's reproducer into a failing native test for the new
   event: two queued intents for X then Y. Assert that both coordinates survive,
   and that Undo removes Y then X.
2. Add the event, the intent kind and the resolution step in `pump`:
   - resolve, then enforce `phase == Commit` (a preview is retired with a clear
     reason);
   - set `base_revision` from the snapshot;
   - continue as a normal commit.
   `Unchanged` and `Retire` settle immediately and `pump` continues to the next
   intent in the same call.
3. Add the remaining tests:
   - a resolver observes the previous commit's document;
   - a vanished target retires with the resolver's reason;
   - `Unchanged` settles with a landing at the current snapshot and no Core effect;
   - a resolver whose command Core rejects settles `Rejected` with Core's message;
   - recovery and closing refuse the event;
   - an epoch change (reopen) rejects queued intents without calling their
     resolvers;
   - a gesture commit queued ahead is not disturbed, and the intent behind it
     resolves against the gesture's result.
4. Leave `Event::Edit` behaviour unchanged. The existing test asserting that its
   base revision is refreshed must still pass.
5. Make the reproducer in `docs/investigations/` point at the new permanent test
   (one line in the investigation doc). Don't delete the reproducer.

## Acceptance criteria

- [ ] Native tests above pass against the real `CoreEngine`.
- [ ] The X-then-Y regression test asserts the final accepted position and both Undo steps.
- [ ] Resolvers are called at most once, only at execution time, and never for intents that are rejected before execution.
- [ ] No change to `Event::Edit`, gesture, export-commit or electrical-remap behaviour (existing tests unchanged and green).
- [ ] The `application` crate's public docs describe the purity rule for resolvers and when they run.

## Verification

```sh
cargo test -p boardstudio-application --locked
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py typecheck
```

## Out of scope

- Any web or presentation change (tickets 04 and 06).
- Making `Event::Edit` stricter ([decision ticket 18](18-decide-restrict-event-edit.md)).

## Pitfalls

- Don't call the resolver in `enqueue`. The whole point is that it runs against
  the snapshot current at execution.
- After an `Unchanged` or `Retire` settles, `pump` must keep draining the queue.
  Otherwise the next intent waits forever.
- Keep `Event` deriving `Clone, Debug, PartialEq`. Implement those traits
  manually on the resolver wrapper.
