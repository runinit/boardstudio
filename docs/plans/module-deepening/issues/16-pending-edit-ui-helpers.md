# 16: Shared UI helpers present pending edits

Status: claimed
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

- [ ] Tests first fail at the helper interface, using real Runtime gates.
- [ ] Mounted tests cover draft retention, latest field value, accepted-value restore,
  inline failure, silent retirement and one-shot disable/re-enable.
- [ ] Landed revision and failure placement remain available to consumers.
- [ ] Native tests exercise plain helper logic where applicable; WASM tests prove
  actual Signal lifecycle behavior. State which tests really execute under each cfg.
- [ ] Outcome documents usage for the final Matrix integration and later panel waves.

## Verification

```sh
cargo test -p boardstudio-web-ui-shared --locked
python3 scripts/check.py lint typecheck test browser
```

Execute both UI-shared browser groups, including the new helper tests. Serialize all
Chrome runs through the orchestrator's lease. Complete parallel Standards/Spec review.
