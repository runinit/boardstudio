# 137: Explain the PCB blocker in Export

Status: resolved
Type: build
Blocked by: none

Claimed by: E coordinator `01a1276a-5631-7b73-a8d3-9d864b586f5f`. Implementation and fresh independent review completed; root integration follows.

## Problem

The retained source critique identifies a KiCad Export row that requires PCB and wiring readiness but selects its reason only from `wiring_blocker`. With PCB unready and wiring ready, the disabled “Needs work” row can have no explanation. Treat this as a hypothesis until a compiling regression fails for that missing reason; no live user failure is claimed.

Evidence: /home/chris/.cache/boardstudio-ui-critique/assessment-A-design.md, priority issue1; retained source `web/src/presentation/export_workspace.rs:271` and disabled-row rendering at170. Rebind current source before editing; the critique is historical evidence, not a fresh index claim.

## Bounded implementation and acceptance

- Reproduce PCB-unready/wiring-ready with a meaningful regression for the visible row explanation; retain its disabled export guard. Then use the existing authoritative PCB blocker when wiring is ready. Preserve existing wiring-blocker and fully-ready behavior; do not change readiness policy or add a new recovery flow.
- Source/test lease: `web/src/presentation/export_workspace.rs` only. Root authorized a narrow inline regression module in this already-leased file because no existing Export test home exists. Follow existing conventions and prove a meaningful fixture with actual compiled WASM RED; no new file, harness or abstraction.
- Existing E worktree: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-inspector-copy`, branch `codex/inspector-copy`; root reports clean integration sync. Root binds actual HEAD/index and overlaps before START. No Sofle, board adaptation, hardware, shared Core/API/schema, unrelated styling or configuration work.
- Luna6 medium implements one completed unit, then STOPs. One different fresh Sol6.1 medium reviews the regression/fix and source-bound writer evidence under its own lease; reuse sufficient checks and rerun targeted verification only for a risk/evidence gap. Intermediate RED/fix remains in the writer unit.
- Follow current agent workflow and UI instructions. Run affected repo/lint/typecheck/browser checks, including actual WASM test compilation for presentation coverage; native-only success is insufficient. Root owns refresh, complete change analysis, integration and the appropriate full integration batch. Preserve unrelated work.

## Current allocation

Only app issue137 is allocated for the next implementation. Parts retry remains an unallocated candidate; compact header identity/save status remains deferred pending rendered reproduction. Shared mirror/link is still required, with106 HUMAN visibility approval HOLD unchanged. Sofle/all board work and test-speed work are deferred by the user; recurring polling automation is PAUSED. This Export ticket is unrelated to the previously rejected Sofle harness-capability ticket idea. The bounded Export fix passed fresh independent Sol6.1 medium review; see Outcome.

## Outcome

Luna6 medium implemented the reason-selection fix in `web/src/presentation/export_workspace.rs`; one fresh independent Sol6.1 medium reviewer passed the exact source SHA256 `0e193336011ff1650b7b4c70eca29d7687142aedbe44539494bbf75a774571b0`. PCB-unready/wiring-ready now explains the existing PCB blocker. Existing wiring reason precedence, fully-ready behavior and `pcb_ready && wiring_ready` disabled guard remain unchanged.

The mounted synthetic regression compiled for WASM and failed for the expected missing visible reason, then passed1/1 in headless Chrome with PCB-blocked, wiring-blocked and fully-ready cases. Reviewer inspected both-blocked precedence in source and independently passed diff-check; sufficient writer repo/lint/typecheck results were reused without another suite. Writer check output is retained in tool history, not separate durable logs.

Evidence: `/tmp/boardstudio-ticket137-red.json`, `/tmp/boardstudio-ticket137-green.json`, and `/home/chris/.cache/boardstudio-baselines/export137/independent-review.json`. Root source/index binding and complete graph delta (11/11 symbols, one file, LOW; no partial/truncated) are in `/home/chris/.cache/boardstudio-orchestration/01a1272d-9690-7342-a509-877cc412ec02/export137-final-source-binding.json` and `export137-final-source-graph.json`. Full integration batch remains root-owned. Preserve this regression during demo retirement: it has no shipped-board dependency.
