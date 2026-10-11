# 137: Explain the PCB blocker in Export

Status: claimed
Type: build
Blocked by: none

Claimed by: E coordinator `01a1276a-5631-7b73-a8d3-9d864b586f5f`. START remains pending root claim commit and explicit file/foreground grant.

## Problem

The retained source critique identifies a KiCad Export row that requires PCB and wiring readiness but selects its reason only from `wiring_blocker`. With PCB unready and wiring ready, the disabled “Needs work” row can have no explanation. Treat this as a hypothesis until a compiling regression fails for that missing reason; no live user failure is claimed.

Evidence: /home/chris/.cache/boardstudio-ui-critique/assessment-A-design.md, priority issue1; retained source `web/src/presentation/export_workspace.rs:271` and disabled-row rendering at170. Rebind current source before editing; the critique is historical evidence, not a fresh index claim.

## Bounded implementation and acceptance

- Reproduce PCB-unready/wiring-ready with a meaningful regression for the visible row explanation; retain its disabled export guard. Then use the existing authoritative PCB blocker when wiring is ready. Preserve existing wiring-blocker and fully-ready behavior; do not change readiness policy or add a new recovery flow.
- Source lease: `web/src/presentation/export_workspace.rs` only, plus its existing adjacent/inline regression test. E must confirm the exact existing test path before any test edit and root must include it in the explicit lease; no blanket test lease or new harness/framework.
- Existing E worktree: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-inspector-copy`, branch `codex/inspector-copy`; root reports clean integration sync. Root binds actual HEAD/index and overlaps before START. No Sofle, board adaptation, hardware, shared Core/API/schema, unrelated styling or configuration work.
- Luna6 medium implements one completed unit, then STOPs. A different fresh Sol6.1 xhigh independently verifies the regression/fix and affected checks under its own foreground lease. Intermediate RED/fix remains in the writer unit.
- Follow current agent workflow and UI instructions. Run affected repo/lint/typecheck/browser checks, including actual WASM test compilation for presentation coverage; native-only success is insufficient. Root owns refresh, complete change analysis, integration and the appropriate full integration batch. Preserve unrelated work.

## Current allocation

Only app issue137 is allocated for the next implementation. Parts retry remains an unallocated candidate; compact header identity/save status remains deferred pending rendered reproduction. Shared mirror/link is still required, with106 HUMAN visibility approval HOLD unchanged. Sofle/all board work and test-speed work are deferred by the user; recurring polling automation is PAUSED. This Export ticket is unrelated to the previously rejected Sofle harness-capability ticket idea. No production changes or checks have run for137.
