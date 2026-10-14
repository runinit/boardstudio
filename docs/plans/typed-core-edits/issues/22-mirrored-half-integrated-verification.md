# 22: Mirrored halves integrated verification

Status: ready-for-agent
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

- [ ] Evidence recorded with commands and results; gaps listed honestly.

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


## Progress (2026-10-14, current-tree full gate)

`python3 scripts/check.py` passed all steps in **202.4 s** (repo 0.4, tooling 3.8, lint 4.2, build 53.9, test 29.2, browser 110.8). It executed all 1,976 baselined native tests plus 6 additional tests, 15 CAD browser checks, and 97 selected presentation/case tests. The reused-PCB mounted create/edit/exception/Undo/Redo/save-reopen workflow and Standards, Spec, and plan-wide reviews are still outstanding; acceptance remains open.
