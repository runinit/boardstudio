# 16: Shared UI helpers present pending edits

Status: resolved
Type: build
Blocked by: 15
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## Owner reservation

Reserved for the user's separate AI tool (user, 2026-10-07). Agents in this chat must
not claim or implement this slice. Reservation is not an early implementation claim:
after keyed settlement merges, the orchestrator claims this ticket and provides its
final interface, base commit and dedicated worktree/branch for the external tool.

## What to build

Implement the Signal-bound text-field and one-shot helpers over the published
[PendingEdits interface](15-keyed-pending-edits.md). Own a new module under
`web/crates/ui-shared/src/` and its export in `ui-shared/src/lib.rs`, plus interface
tests. The crate already has a test-support Runtime dev dependency; avoid new
production dependencies. Do not edit Runtime or Matrix files in this slice.

This is a suitable separate-AI-tool work stream after the keyed collection merges.
It runs in parallel with [Matrix one-shot actions](17-matrix-one-shot-pending-edits.md).

## Interface and invariants

- The field helper preserves the current draft while its latest key is pending. On
  settlement it restores/projects the latest accepted value; failure is inline by
  default and retirement is silent. A newer draft must not be replaced by an older
  ticket's outcome. Domain projection/value conversion remains the caller's job.
- The one-shot helper exposes pending/disabled state and returns failure information
  for the panel to place. Landed remains available for caller follow-ups.
- Helpers bind Signals to collection outcomes; they do not duplicate terminal mapping,
  Scope policy, resolver rules or a Saved state. Keep a small interface and avoid
  introducing a general form framework.

## Acceptance criteria

- [x] Tests first fail at the helper interface, using real Runtime gates.
- [x] Mounted tests cover draft retention, latest field value, accepted-value restore,
  inline failure, silent retirement and one-shot disable/re-enable.
- [x] Landed revision and failure placement remain available to consumers.
- [x] Native tests exercise plain helper logic where applicable; WASM tests prove
  actual Signal lifecycle behavior. State which tests really execute under each cfg.
- [x] Outcome documents usage for the final Matrix integration and later panel waves.

## Verification

```sh
cargo test -p boardstudio-web-ui-shared --locked
python3 scripts/check.py lint typecheck test browser
```

Execute both UI-shared browser groups, including the new helper tests. Serialize all
Chrome runs through the orchestrator's lease. Complete parallel Standards/Spec review.

## Outcome

Integrated the second-app commit `a2faaaef4cb704db9526c88e802d6fa008d970f8`.
The clean branch changed only the new helper module and its lib export. Final pinned
Standards and Spec re-reviews found no blocking findings. Spec confirmed that the
Matrix migration can rehome its action collection under the helper without another
collection or adapter; preserving an allocation across recompilation is not required.

`PendingEditSignals<K: PartialEq + 'static>` clones share one keyed collection. Its
interface binds fields (`bind_field(key, draft, failure)`) and actions
(`bind_one_shot(key, disabled)`), submits through `begin_field`/`begin_one_shot`,
answers `is_pending(&key)`, and drains `settle(owner_is_live, accepted_projection)`
as the existing `PendingEditResult<K>` variants. Field submission additionally needs
Clone keys and records the submitted text. The helper owns draft bookkeeping and
Signal writes; accepted-value projection and exact selection remain caller-owned.
Landed and Retired restore only an untouched submitted draft; newer drafts survive.
Failure is inline and also returned for other placement; retirement is silent.
There is no Saved state, retry, second terminal mapping or Scope policy.

Consumer shape:

```rust
let helpers = use_hook(|| PendingEditSignals::<MatrixPendingKey>::new());
helpers.bind_field(MatrixPendingKey::Rows, rows, rows_failure);
helpers.bind_one_shot(MatrixPendingKey::Preset(None), preset_disabled);
helpers.begin_field(&runtime, MatrixPendingKey::Rows, "matrix-rows",
    Some("matrix".into()), resolver, &rows.peek().clone());
for result in helpers.settle(owner_is_live, |key| accepted_text(key)) {
    // Retain request-bearing action keys for exact Landed follow-ups.
}
```

The external app reports red-before-green interface evidence, native UI-shared 8/8,
lint/typecheck/fmt/Clippy passing, and browser groups 16/16 plus panels 1/1. Seven new
mounted tests use real WASM Runtime gates and inputs/buttons, covering draft retention,
latest replacement, accepted restoration, failure, newer drafts, Unchanged landing,
Scope retirement with a live owner, departed-owner retirement and one-shot state.
Native cfg executes the pure draft policy test; mounted behavior executes in Chrome.
Root reused this final evidence and inspected the final committed source.

The app's full browser step exposed native Runtime collection fixtures compiling for
WASM; root independently reproduced that test-target failure and integrated the native-only
fixture cfg fix recorded in the keyed collection Outcome. It does not invalidate the directly executed UI-shared browser groups.
Its full native step stopped at 13 KiCad environment failures: a missing pcbnew library
under ZCode's AppImage mount. An affected KiCad test passed from root's clean process
on the same tree; these are reported separately from the CAD volume baseline, and
are not evidence that the full test step passed.

Bindings/draft records last for the helper lifetime. Use bounded logical panel keys;
Matrix action keys compare by action kind even when carrying request metadata. Do not
introduce unbounded per-operation keys. The next consumer removes its direct action
collection and uses one helper-owned collection for field and action observations.
