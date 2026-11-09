# 14: Edit tickets own captured Scope liveness

Status: resolved
Type: build
Blocked by: 03, 04
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

Make `EditTicket` answer document/board/instance lineage liveness itself. This is the
first slice after the resolution constructors merge; it does not migrate panel
settlement or add the keyed collection yet.

Own `web/crates/runtime/src/edit_ticket.rs` and the Runtime scope-port adapter there.
Change Runtime support files only if the read-only scope source requires it; keep any
`runtime.rs` change in its own small first commit. Keep the existing panel call surface
usable so every intermediate branch compiles without a sweep of workspace files.

## Interface and invariants

- The ticket captures the current `Scope` before submission, and can read the current
  scope through its port at observation. The implementation chooses the ownership of
  that read-only source; callers do not carry a second captured scope for settlement.
- A scope mismatch retires the observation, even when its authoritative operation is
  still pending or has landed. A retired observation cannot resume as live after the
  panel returns. Dropping/retiring observation does not cancel the Session operation.
- `owner_is_live` describes only the panel's selection/mount lifetime. Accepted revision
  and token changes inside the same Scope do not retire a queued edit.
- Observe before submit, preserve failure wording and landing revision, and use the
  real native Runtime from the native Runtime ticket. Do not fabricate scope changes,
  replace a Session under an active gate, or add a production native Runtime.

## Acceptance criteria

- [x] Native interface tests fail first for captured-scope retirement, then pass.
- [x] Held Core/save operations demonstrate retirement on real navigation or Open;
  use actual Session sequencing, including Open/Close draining active work.
- [x] Revision changes in the same Scope remain live; departed panel owners retire.
- [x] Existing ticket outcomes and Runtime consumers continue to compile and pass.
- [x] The port/lifetime contract is documented for the keyed collection implementer.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py lint typecheck test
```

Run the affected Runtime browser suite for changes to the WASM port. Record known
baseline gate failures separately; test-only native changes need no unrelated browser
rerun. Complete the handoff's parallel Standards and Spec reviews before integration.

## Outcome

Merged `f0c5cb1f70a66dd485ab3e4d0289dfd13adc0985` into dev at `b19bdc9f8`.
Both pinned Standards and Spec reviews passed. The retirement regression failed first
with the old always-live behavior, then passed. Native Runtime: 105/105; Runtime
browser: 29/29; lint and typecheck passed. Workspace and footprint tests passed;
the existing rotated-concave bottom CAD baseline remains 49 passed, 1 failed,
4 ignored (expected 80481.2399, actual 80579.55733514718, tolerance 0.1).

`EditTicketPort::scope_source()` returns an owned `Rc<dyn Fn() -> Option<Scope>>`.
`EditTicket::begin(port, label, feature, resolver)` captures Scope before allocation,
observation and submission. Ticket clones share the reader, outcome/landing slots and
a retirement latch. `settlement(owner_is_live)` uses the predicate only for panel
lifetime; Scope mismatch or owner departure permanently retires the observation.
`is_pending()` also checks Scope liveness. Same-Scope revision changes remain live.
Retirement/drop does not cancel the authoritative Session operation.

Real navigation away and back to the exact original Scope cannot revive an observed
retirement, including across clones. Actual Open drains active work before changing
Scope. Close also drains active work: while its accepted Scope remains unchanged,
the active edit can report Landed; no lifecycle-only retirement policy was added.
The keyed collection can consume this port without a second captured Scope.
