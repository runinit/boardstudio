# 22: Mirrored halves integrated verification

Status: resolved
Type: task
Blocked by: [21](21-mirrored-half-web-adapter.md), [41](41-inspector-companion-mode.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Verify the native mirrored-half feature as a whole using independently authored synthetic
documents. Shipped/demo boards and their fixtures were retired by [138](138-retire-shipped-demos.md)
and are not acceptance inputs.

## What to do

- Boundary transcript native and WASM; the full `python3 scripts/check.py` (never `release`).
- Mounted browser pass on synthetic linked halves, including a reused-PCB case: create, edit
  from each half, local exception, Undo/Redo, save and reopen; record anything not run. Define
  geometry and expected results independently; do not restore or copy demo inputs.
- Standards and Spec reviews of Core, web and data changes; plan-wide review.

## Acceptance

- [x] Evidence recorded with commands and results; gaps listed honestly.

## Progress (2026-10-14)

`python3 scripts/check.py` passed all steps on `dev` in 272.3 s (repo 0.4, tooling 4.8, lint 4.5,
build 106.3, test 37.3, browser 119.1). This includes the native/WASM boundary transcript checks,
15 CAD browser checks, and 96 page WASM tests. The synthetic linked-half mounted workflow with a
reused PCB, save/reopen, and the required Standards, Spec, and plan-wide reviews have not been run.
Ticket 22 remains open.

## Progress (2026-10-14, full gate after Rejoin coverage)

`python3 scripts/check.py` passed on the current tree in 202.3 s (repo 0.4, tooling 3.9, lint 3.6,
build 46.6, test 28.3, browser 119.5). This run includes 97 presentation and case-workspace WASM
tests. The reused-PCB save/reopen workflow and required reviews remain outstanding; acceptance stays
open.

## Progress (2026-10-14, current tree full gate)

`python3 scripts/check.py` passed all steps on the current tree in 203.8 s (repo 0.4, tooling 3.8,
lint 3.7, build 49.4, test 29.1, browser 117.3). The page WASM run executed 97 tests with no
failures. The reused-PCB create/edit/exception/Undo/Redo/save-reopen workflow and required reviews
remain outstanding, so acceptance stays open.

## Progress (2026-10-14, full gate after export wait conversion)

`python3 scripts/check.py` passed all steps on the current tree in 206.6 s (repo 0.4, tooling 3.8,
lint 3.6, build 48.8, test 28.7, browser 121.2). All 97 page WASM tests passed. The reused-PCB
save/reopen workflow and required reviews remain outstanding; acceptance stays open.

## Progress (2026-10-14, current Layout wait conversions)

`python3 scripts/check.py` passed all steps in 201.6 s (repo 0.4, tooling 3.8, lint 3.7, build 49.5,
test 27.9, browser 116.2). All 97 page WASM tests passed. The synthetic reused-PCB workflow and
required reviews remain outstanding; acceptance stays open.

## Progress (2026-10-14, component Inspector wait conversion)

After commit `8dc4634c`, `python3 scripts/check.py` passed all steps in **195.1 s** (repo 0.4,
tooling 3.8, lint 3.5, build 46.4, test 28.8, browser 112.3); all 97 page WASM tests passed.
The synthetic reused-PCB create/edit/exception/Undo/Redo/save-reopen workflow and required
Standards, Spec, and plan-wide reviews remain outstanding. Acceptance stays open.

After the ui-shared test-wait changes, `python3 scripts/check.py` passed all steps in **207.3 s**
(repo 0.4, tooling 4.0, lint 4.6, build 54.3, test 31.7, browser 112.3); all 97 page WASM tests
passed. This refreshes the repository gate; the synthetic reused-PCB workflow and required reviews
remain outstanding.

## Outcome (2026-10-14)

The mounted WASM test `mounted_reused_pcb_pair_undo_redo_and_reopen` in
`web/src/presentation/layout_component_inspector_tests.rs` opens an independently authored,
source-only synthetic half, creates its mate with typed `CreateMirroredHalf` in `ReuseBoard` mode,
and verifies one shared board, pair creation Undo/Redo, memory save and fresh-runtime reopen.
Focused command: `TMPDIR=/home/chris/.cache/bstmp wasm-pack test --headless --chrome web --locked
--no-default-features --features page --bin boardstudio-web --
mounted_reused_pcb_pair_undo_redo_and_reopen` — passed (1 test). The existing mounted
`mounted_mirrored_half_actions_work_from_either_board_and_undo` test separately exercises
linked/local/omitted Inspector actions and Undo from both distinct board scopes.

The shared PCB has no distinct Part correspondence, so a per-Part local exception does not apply
to it. The reusable half does have independently owned mechanical features: the test edits a
linked mount from each instance, makes the mirror mount local through `ApplyMechanicalEdit`, checks
Undo/Redo of that exception, then edits the local feature without changing the source. Save/reopen
preserves the one-board pair, local feature state and local geometry. The test submits typed Core
commands through the mounted Runtime; it does not drive the mechanical feature control in the
Inspector from both instance scopes.

Review fixed point: `d06bef8a61195e9f8dc152ad19ae8472fa8339f9`. Standards review found no documented
standard breach; one possible duplicated argument list in `create_with_physical_setup` was noted
as a judgement call. Spec review found no confirmed #21/#40/#41 behavior mismatch. Plan-wide review
confirmed the related mirror tickets are resolved and their synthetic-only scope is current. The refreshed Standards and Spec reviews cover the added assertions; no documented-standard breach
or confirmed spec failure was found. The mounted test uses typed Runtime/Core commands rather than
clicking the mechanical Inspector control in both instance scopes; this is the remaining evidence
limit, and the ticket does not require that control path explicitly.

`cargo fmt --all --check` and `git diff --check` passed. The final
`TMPDIR=/home/chris/.cache/bstmp python3 scripts/check.py` passed all steps in **196.6 s**:
repo 0.4, tooling 4.0, lint 3.6, build 47.1, native tests 28.5, browser 113.0. Boundary transcript
checks, 1,976 baselined native tests (+6 new), 15 CAD browser checks and 99 selected page/case
WASM tests passed.


## Resolution (2026-10-14)

The reused-PCB workflow, current full gate and refreshed Standards/Spec reviews satisfy this
ticket. The remaining evidence limit is that mechanical edits use typed commands through mounted
Runtime/Core rather than clicking the Inspector control from both instance scopes. The separate
mounted Inspector test covers linked/local/omitted actions from both distinct-board scopes.
