# 32: Optional labels for the source and the mirror

Status: resolved
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-companion-modes-pads-labels-2026-10-10)

## Request

User, 2026-10-10: Core does not refuse a source that is neither left nor right; it provides a rename option. Ticket
[26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md) found that a unibody source gives a mirror labelled
half `unibody`, name "Keyboard mirror", role `central`.

## What to do

- Add optional `labels` to the `CreateMirroredHalf` request: `{source: {name, half, role}, mirror: {name, half, role}}`,
  each field optional; absent keeps today's behaviour (source untouched, mirror labelled by the existing rule).
- Validate (non-empty names, known half/role values, the two halves must not both claim the same half and role) with
  located errors and no mutation. Applying labels is part of the same one Undo step.
- Tests: unibody source with labels gives `Left half`/`Right half`; no labels keeps today's result; invalid labels refused.
- Update ticket [21](21-mirrored-half-web-adapter.md): the adapter passes the labels (physical setup's relabelling) instead of a separate edit.

## Acceptance

- [x] Labels applied atomically with the creation; unlabelled behaviour unchanged.
- [x] The ticket 26 equivalence test shrinks its pinned difference list by the relabelling rows when labels are passed.

## Verification

`repo lint test typecheck`; wasm boundary parity if the transcript changes.

## Outcome

Implemented by Codex, 2026-10-10, on `lane/mirror-core` in `.worktrees/lane-mirror-core`.
Claim committed as `9f6c400d`; the implementation is committed with this Outcome.

- `CreateMirroredHalf` accepts optional `labels.source` and `labels.mirror`; each accepts optional
  `name`, `half` and `role`. Missing fields retain the source's stored labels or the mirror's existing
  defaults, computed before relabelling. No label is stored as a pair setting or reflected later.
- Labels apply to the cloned creation draft in both `new-board` and `reuse-board`. Names must contain
  non-whitespace text; supplied halves are `left`, `right` or `unibody`, and roles are `central`,
  `peripheral` or `standalone`. With labels supplied, the resulting instances cannot have both the
  same half and the same role. Errors locate the pair, instance and `labels.<side>.<field>` (or
  `labels.mirror` for a duplicate claim); a refusal discards the draft.
- Four Core request tests cover a unibody source, both modes, partial/empty/omitted labels, located
  refusals with byte-identical accepted snapshots, preview isolation, changed instance ids, reopen,
  and one-step Undo/Redo. Before implementation, three failed because labels were ignored and invalid
  labels accepted; the default-behaviour test passed. All four pass after implementation.
- Ticket 26's equivalence test supplies physical setup's left/central/“Left half” and
  right/peripheral/“Right half” labels. Only its six relabelling rows disappear; the exact remaining
  four differences (`halves`, `sharedConstruction`, `topology`, `transport`) and all feature/closure
  assertions remain. The six label fields are also asserted present and equal. Ticket 21 now passes
  labels in this request instead of a separate relabelling edit; its adapter implementation remains
  ticket 21's work.

### Verification

- `cargo test -p boardstudio_core --test integration creation_labels -- --nocapture`: four passed
  after the expected pre-implementation failures (three failed, one passed).
- `CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target python3 scripts/check.py repo lint test typecheck`:
  all passed; repo 0.1 s, lint 24.3 s, test 109.7 s, typecheck 29.5 s, total 163.6 s. Core integration:
  557 passed, including native boundary parity and the four new label tests. The catalogue equivalence
  test passed. Existing ignored tests remain ignored; existing dependency/page warnings remain.
- `git diff --check` passed. `repo` rerun after the final ticket/map updates.
The boundary transcript and fixtures are unchanged; the conditional WASM parity run is not needed.

GitNexus was refreshed in this lane at claim commit `9f6c400d`. Pre-edit impact rated `create` and
`apply` CRITICAL through the Core edit/preview flows (warning reported before implementation);
`create` has the dispatcher and a unit test as direct callers. `EditOperation` and test entry points
returned UNKNOWN; text searches confirmed the variant's dispatch/construction sites and test harness
entry points. The catalogue helper has one direct caller, the equivalence test (LOW). Full MCP
`detect_changes` was used after the CLI's abbreviated display; no partial/truncated flag.
