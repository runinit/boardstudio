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

Verified by Claude Sonnet 5.5, 2026-10-10, in `.worktrees/lane-mirror-core` (Codex implemented). Reviewer pass; the
implementation needed no code change. Machine shared with a browser run (`uptime` load average 3.2 at the start,
7 to 8 during `repo`/`lint`/`test`, 15.6 at the end), so timings are indicative only. GitNexus was not used (stale);
callers and sites were read with `rg` and the diff.

Process corrections made in this lane (commit `56a980a0`, before this note):

- Codex edited `docs/plans/typed-core-edits/map.md` and set this ticket `resolved` itself. Lanes do not edit `map.md`
  and implementers never set `resolved`. The `map.md` edit is reverted to its content before the claim commit; the
  progress line for ticket 32 is for the orchestrator to add at merge time (suggested text: "32 optional
  source/mirror labels apply atomically with creation; located validation; one Undo step; ticket 26's six relabelling
  rows removed").
- Codex's Outcome says it refreshed GitNexus in this lane at the claim commit and updated the map. Lanes must do
  neither (see [parallel-lanes.md](../parallel-lanes.md)); no harm found, recorded for the orchestrator.
- Ticket 21's edit is a correct pointer (adapter passes `labels`; the other four rows stay pinned) and stays.

### What was checked and observed

- **Request and validation (read from `core/src/mirrored_half.rs:88-180`, `core/src/model.rs:1162-1182`, `core/src/lib.rs`).**
  `labels: {source, mirror}` each `{name, half, role}`, all `Option` with `#[serde(default, skip_serializing_if)]`; the
  single-word fields need no rename, `labels` sits beside `pairId`/`mode` in the kebab-case-tagged enum. Applied to the
  cloned draft `next` after the mode builds the mirror and before `layout::validate`/`halves::validate_document`, so
  a refusal drops the draft and success commits the pair and both relabellings as one accepted edit. Errors name the
  pair, instance and `labels.<side>.<field>` (`labels.mirror` for a duplicate claim). No format bump is needed:
  `labels` exists only on the request and is never stored (the document schema and `hardware` shape are unchanged), and
  the field is optional, so older requests parse and serialise identically (v5 is unreleased in any case).
- **Unibody question.** `mirror_half_label` is unchanged: the mirror is derived from the source's labels as they were
  at creation, and the supplied labels then overwrite. So a unibody source with both labels gives `Left half` /
  `Right half`; a unibody source with no labels, or only partial labels, keeps today's `unibody`, role `central`,
  "Keyboard mirror" mirror (documented in a code comment and pinned by the partial-label test). The duplicate
  half-and-role check runs whenever `labels` is present, even `{}`, so `labels: {}` on a source that is already
  `unibody`/`central` is refused while omitting `labels` is accepted (see follow-up 2; derived by reading, not run).
- **Equivalence test (`physical_setup.rs`).** Still compares the two real paths (`physical_setup_split` against
  `CreateMirroredHalf` ReuseBoard through `CoreEngine`). The six relabelling rows left the pinned list and moved into
  the must-be-present-and-equal pointers; the four remaining rows (`pairs/0/halves`, `sharedConstruction`,
  `topology`, `transport`) are exactly the ticket 26 rows 7 to 10 and still pinned. The no-label behaviour is no
  longer exercised in the catalogue test (the test only runs the labelled request); it is pinned in Core by
  `omitted_and_empty_creation_labels_preserve_unibody_defaults` (both modes; omitted, `{}`, `{source:{},mirror:{}}`).
- **Atomicity.** `refused()` in the tests snapshots the engine before and after and compares the JSON strings (byte
  identical), for nine invalid label shapes in both modes. Undo restores both labels in one step and Redo returns the
  made document (both modes). Preview does not mutate. Creation without labels takes no new code path (the
  `if let Some(labels)` is skipped) and `labels` is not serialised when absent.
- **Boundary transcript.** `git diff 9f6c400d6^ HEAD --stat -- core/tests/fixtures` is empty: unchanged. The wasm
  `boundary_parity::` run was not needed and not run; `typecheck` compiles the new type for the page.

### Commands (sandbox lifted for cargo; `target` symlink is outside the sandbox write list)

- `cargo test -p boardstudio_core --test integration mirrored_half::`: 41 passed, 17.1 s test time (about 25 s with
  an incremental rebuild).
- `cargo test -p boardstudio-web-catalogue --lib physical_setup_and_reuse_board_agree`: 1 passed.
- `python3 scripts/check.py repo`, `lint`, `test`, `typecheck`, run once each, sequentially: all passed. repo 0.1 s,
  lint 20.5 s, test 92.2 s (Core integration included), typecheck 31.6 s.

### Mutation checks (each applied, run, restored; sources touched and `git status` clean afterwards)

Core `mirrored_half::` (41 tests), by disabling one site at a time in `apply_label`:

| Disabled | Result |
| --- | --- |
| source name | 3 failed (`left.name`) |
| mirror name | 2 failed (`right.name`) |
| source half | 3 failed (`left.half`) |
| mirror half | 3 failed (`right.half`) |
| source role | 3 failed (`left.role`) |
| mirror role | 3 failed (`right.role`) |
| duplicate half-and-role check | 1 failed (`invalid_creation_labels_...`) |
| empty-name check | 1 failed (same test) |
| half value check | 1 failed (same test) |
| role value check | 1 failed (same test) |
| labels not applied at all | 3 failed |

Catalogue equivalence test: Core mirror name skipped fails with `/hardware/instances/1/name differs`; Core source role
skipped fails with `/hardware/instances/0/role differs`; removing the mirror role from the test's request fails with
`/hardware/instances/1/role differs`.

### Acceptance

Both acceptance items hold, so the ticket is `resolved`.

### Follow-ups and unverified (none block this ticket)

1. The catalogue test no longer runs an unlabelled request. Core pins the unlabelled values, but if ticket 21
   wants the F1 defect visible at the adapter boundary it can add a second unlabelled run of the helper.
2. `labels: {}` (present but empty) refuses a source that already shares half and role with its default mirror
   (for example `unibody`/`central`), while omitting `labels` accepts it. If that inconsistency matters, skip the check
   when no field was supplied. Not changed: it is a behaviour choice and a corner of a corner.
3. Names are stored as given (only an all-whitespace name is refused, not trimmed). Cosmetic.
4. Not run: wasm `boundary_parity::` (transcript unchanged), `browser`, `build`. Undo/Redo is shown in Core only; the
   adapter's Undo of the combined edit is ticket 21's test.

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
