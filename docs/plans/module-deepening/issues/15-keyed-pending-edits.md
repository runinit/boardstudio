# 15: PendingEdits owns keyed settlement

Status: resolved
Type: build
Blocked by: 14
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

Add the Dioxus-free keyed collection in `web/crates/runtime/src/pending_edits.rs`,
with its export in `runtime/src/lib.rs`. Use the scope-aware ticket from
[Edit tickets own captured Scope liveness](14-edit-ticket-scope-lineage.md).
Keep this slice inside Runtime; do not migrate the Matrix controller or UI helpers.

## Interface contract

Exact Rust names and generic bounds are the implementer's choice, reviewed with
`codebase-design`; publish the tested contract in this ticket's Outcome before the
parallel consumers start.

- `begin(port, key, label, feature, resolver)` starts and observes an edit. The latest
  ticket per key replaces the previous observation; previous Session edits still run.
- `is_pending(&key)` supports one-shot admission until the observed edit settles or
  retires. One key does not block another field key.
- `settle(owner_is_live)` drains terminal observations and returns keyed `Landed`
  (with revision), `Failed` (with message), or `Retired` results. Pending observations
  remain tracked; a terminal result is returned once. The owner predicate answers
  only panel lifetime; the ticket owns Scope liveness.
- There is no Saved status, no presentation state and no automatic retry. Retirement
  carries no failure message. Cloning or Signal storage must not acknowledge an old
  ticket as the replacement ticket's result.

## Acceptance criteria

- [x] Native tests use real Runtime/Core/save gates at the collection's interface.
- [x] Cover latest-per-key replacement with both Session operations still executing,
  independent keys, and one-shot pending until settled.
- [x] Cover Core/save failure wording, Landed revision, Unchanged landing, and
  exactly-once draining of terminal results.
- [x] Cover Scope change, departed owner, and Closed retirement through actual Session sequencing; do not hand-settle
  Runtime. Superseded/Cancelled mapping remains covered at EditTicket because those
  outcomes are unreachable from the collection's ResolveEdit-only interface (see Outcome).
- [x] Runtime remains Dioxus-free; the new module compiles natively and for WASM.
- [x] Outcome contains a small consumer example and the final interface/invariants;
  shared UI helpers and Matrix actions can start without designing another policy.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py lint typecheck test
```

Run affected Runtime browser tests if the shared ticket/port implementation changes.
Complete the handoff's parallel reviews. Later interface changes require coordination
with both consumer owners rather than independent edits to Runtime.

## Outcome

Merged commits `df72b7b843c7e6db1be77c9bf7bbbe7c750df0a0` and
`1d0724d35b3f1b43ba09fda708b184607caf9671`. Pinned Standards review passed;
Spec found no functional mismatch. This Outcome closes its documentation gap and
records the reachable-outcome clarification below.

The Dioxus-free `PendingEdits<K: PartialEq>` owns a Vec of keyed tickets; keys need
no Clone/Hash/Ord bound. The collection itself is not Clone. Its API is:

```rust
begin(&mut self, port: &dyn EditTicketPort, key: K, label: &str,
      feature: Option<String>, resolver: EditResolver) -> OperationId
is_pending(&self, key: &K) -> bool
settle(&mut self, owner_is_live: bool) -> Vec<PendingEditResult<K>>
```

Results are `Landed { key, revision }`, `Failed { key, message }`, or
`Retired { key }`. The latest observation replaces its key; earlier authoritative
Session edits still execute. Terminal results drain once, pending observations stay,
and key insertion order is preserved. Ticket-owned Scope liveness and the panel-only
owner predicate are reused. There is no Saved, presentation state or automatic retry.

Example (see the module docs for the full interface):

```rust
let mut edits = PendingEdits::default();
edits.begin(&runtime, field_key, "matrix-field", Some("matrix".into()), resolver);
let pending = edits.is_pending(&field_key);
for result in edits.settle(owner_is_live) {
    // Landed: project accepted value and caller follow-up.
    // Failed: project accepted value and place the message.
    // Retired: discard observation silently.
}
```

Five new native tests use real Session/Core/save gates. Runtime passed 110/110;
lint, WASM typecheck, workspace and footprint suites passed. The CAD step reproduced
the known baseline: 49 passed, 1 failed, 4 ignored; rotated-concave/bottom expected
80481.2399, actual 80579.55733514718, tolerance 0.1. Shared ticket/port implementation
was unchanged, so no unrelated Runtime browser rerun was required.

Actual Closed retirement is exercised by submitting after Session closes. ResolveEdit
becomes a non-strict committed edit; Superseded and Cancelled are emitted by other
strict/gesture/job/export routes, not this collection interface. Source verification
confirms that Open/Close drain resolver work. Fabricating those outcomes through a
substitute port would violate the real-Session test requirement. The existing
EditTicket outcome-mapping coverage remains; no Session policy was changed.

GitNexus was stale at agent verification and missed the new unindexed module; source
inspection confirmed the bounded additions. Both consumers must coordinate later API
changes rather than independently edit Runtime.
