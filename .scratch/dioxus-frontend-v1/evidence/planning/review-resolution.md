# Workflow planning review and resolutions

**Date:** 2026-10-02. **Source baseline:** c827c4e6 / executable f44a3d1b; pinned React 5a472a94. This is planning review and source verification, not implementation or browser acceptance. No application source/build/demo changed.

Each workflow had an author and a different agent reviewing its source claims and execution boundaries:

| Workflow | Author agent | Independent reviewer | Report |
| --- | --- | --- | --- |
| F2 | `plan_projects_shared` | `plan_pcb` | [review](F2-review.md) |
| F3 | `plan_layout` | `plan_pcb` | [review](F3-review.md) |
| F4 | `layout_documenter` | `plan_pcb` | [review](F4-review.md) |
| F5 | `plan_pcb` | `layout_documenter` | [review](F5-review.md) |
| F6 | `plan_key_workspaces` | `plan_pcb` | [review](F6-review.md) |
| F7 | `plan_key_workspaces` | `layout_documenter` | [review](F7-review.md) |
| F8 | `layout_documenter` | `plan_key_workspaces` | [review](F8-review.md) |
| F9 | `root` | `plan_pcb` | [review](F9-review.md) |

Root reconciled the plans and generated the combined dependency graph and source ledger. `plan_pcb` separately reviewed the graph and execution plan; [initial graph findings](graph-review.md) and [cross-plan findings](cross-plan-review.md) are retained. The final documentation validator checks the generated result; it does not substitute for future implementation reviews.

## Resolved findings

1. **Broad dependencies:** Layout no longer waits for all F2. Parts catalogue/generator/preview and saved recipe work start independently of unrelated project/custom-footprint/placement work. Exact `start_after` and `acceptance_after` replace coarse or symbolic labels. Removed the F4.3→F4.2 and F4.4→F4.3 acceptance edges called out in graph review.
2. **Viewer ownership and scope:** F7 owns one shared viewer; F4 owns isolated sample inputs. Layout/Keymap/Keycaps use canonical board state; Case uses selected physical-instance state. Existing wasm state/handles/pick/camera operations need crate-private wrappers, not a presumed public renderer expansion. F5.7 finishes reference editing independently; F5.8 joins actual viewer/instance evidence.
3. **Profiles and assets:** F4 owns reusable part/module definition profiles and source HardwareReadiness. F7 owns case-configuration assignments/overrides/extraction. Generic file/asset I/O is coordinator-owned; feature forms consume it.
4. **Reference UI scope:** Remove invented project rename/duplicate and visible export progress/cancel/dedicated retry. Preserve actual error/repeat-action retry behavior and Case generation's real controls. Renderer errors preserve surrounding controls and stop/report cleanly; only existing provider/model retry affordances are required.
5. **Keycap CAD:** The first F8 review correctly found no Dioxus keycap request but overstated the missing provider. Follow-up located the already packaged Rust WASM `build_keycaps`. BND.1 proves a private worker adapter, including preview chunking, IDs, scope/revision validation and non-preemptible STEP late-result suppression. No new CAD algorithm/public export is indicated. The [source trace](boundary-gaps.md) supersedes the initial engine inference.
6. **Export snapshot lineage:** Current Session cancels registered exports on accepted commits and has no self-adopt transition. BND.2 must prove private orchestration of wiring commit→package→protection commit→guarded delivery. Ordinary edits/scope changes remain invalidating. A true public-contract requirement would need a concrete separate decision; it is not assumed away.
7. **Archive and output scope:** F2 shared packing must include local assets, optional bundled used models and the project-name filename. F8 owns its route copy/embedding controls. Authored Case STEP uses canonical selected-board bodies; generated mechanical export uses selected-instance effective geometry. URL error cleanup is retained lifecycle hardening.
8. **Supporting frontend policy:** F3.8 explicitly ports the supported script editor. F6C.3 ports existing TS resize/reflow policy into a private Rust controller; it is not described as an existing Rust endpoint. F3.3 characterizes the React non-zero versus current Dioxus 4px drag mismatch before changing it and preserves accepted pointer/cancel/history/performance gates.
9. **Release gates:** F9 preparation starts alongside implementation; its acceptance joins all real workflows. Actual AT remains host-blocked and production cutover requires the final concrete approval. Planning makes no full-file migration or new milestone-completion claims.

## Remaining implementation work

All 62 task rows remain planned. BND.1 needs a working private keycap adapter proof; BND.2's lineage approach remains a design candidate, not a proven implementation. Actual screen-reader coverage needs a suitable host. These are explicit task/gate records, not unresolved planning omissions or permission requests for this documentation turn.

## User-requested post-port takeaways

Added the living refactoring register with 12 initial observations/risks, independently source-checked by plan_pcb. Every task/author/reviewer handoff now records RF IDs or explicitly states none observed. The additional library-to-binary reachability finding is an F7.1 checkpoint; CAD-only integer limits and reflective renderer typing are retained as later investigation topics. Required fixes stay in the port; structural proposals remain deferred.
