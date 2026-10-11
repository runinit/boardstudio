# Delivery queue

As-of snapshot: checkout /home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix, branch typed-core-edits/mechanical, integration HEAD 6163b9221 (2026-10-11). The publication136 integration cohort passed 382.2 s: repo 0.1, tooling 3.0, lint 6.5, build 56.9, test 65.6 and browser 250.1; log /tmp/boardstudio-cohort-publication136-retry.log. Counts describe recorded materialized ticket status, not a live worker poll. Retained lane readiness is labeled separately; ticket bodies and maps remain authoritative for acceptance and dependencies.

## Effort inventory

| Effort | Status-bearing tickets | Pending | Recorded frontier |
| --- | ---: | ---: | --- |
| [Check speed](check-speed/map.md) | 13 | 3 | 11 `ready-for-agent` (blocked by resolved typed-core 33); 12 `ready-for-agent` after 11 and 34; 13 `needs-info` after 12. |
| [Edit settlement](edit-settlement/map.md) | 20 | 0 | Complete; ticket 20 is resolved. |
| [Generator parameters](generator-parameters/map.md) | 6 | 0 | Complete. Keyboard refresh 04 belongs to typed-core, not this effort. |
| [Matrix layout consistency](matrix-layout-consistency/map.md) | 12 | 0 | Complete; ticket 12 resolved after independent review and the E12+F135 integration cohort. Remaining visual observation is separate, unallocated work. |
| [Module deepening](module-deepening/map.md) | 19 | 0 | Complete; no open status-bearing tickets. |
| [Typed Core edits](typed-core-edits/map.md) | 128 | 34 | Open tickets span several serial/blocked streams; see ranked actions and full linked inventory. The unnumbered `04-handoff.md` note has no Status header and is excluded. |

There are 198 status-bearing tickets: 37 pending (34 typed-core and 3 check-speed), plus 55 typed-core tickets marked wontfix. IDs120–127 are reserved without materialized ticket files. Completed119 and128–136 are excluded from pending counts.

## Ranked next actions

### 1. B: preserve bounded 106 implementation evidence; full acceptance remains open

**Recorded tracker status:** [106](typed-core-edits/issues/106-reconcile-mirrored-matrices-layouts-parts.md) remains claimed/open; [134](typed-core-edits/issues/134-reconcile-fixed-membership-matrix-assemblies.md) is resolved and integrated. **Root-retained state:** core/src/mirrored_half/reconcile.rs at 3b1a17548b8ae392435bf124a70324f9d0ca2471d4ae8a11eb0aad31e97ba575; focused 44/44 PASS including retained eight regression refusals and independent probe-only net-oracle, distinct Local nets and nondefault preservation fields. B explicitly STOPPED/released. Cleanup wrapper requiring new-public-crate visibility awaits HUMAN approval. This is not full106 acceptance or closure; parents106/19 remain open. Evidence: /home/chris/.cache/boardstudio-baselines/ticket106-transition/validated-context-implementation/focused-reconcile-tests.log.

### 2. F: publication complete; next planned assembly-evidence unit is on HOLD

**Recorded tracker status:** Ticket [136](typed-core-edits/issues/136-saved-project-publication.md) is resolved and integrated at 6163b9221. Independent Sol 6.1 xhigh publication review passed six exact-restored meaningful mutations, native 2/2, Chrome Library 16 and complete source-bound graph MEDIUM (14/14 symbols, four files). Full integration cohort passed 382.2 s; log /tmp/boardstudio-cohort-publication136-retry.log. Binding: /home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-final-source-output-binding.json. Accepted archive remains SHA256 a2509dd893761b9a85ab0ba7085f701653cd8cbe77120de8844c5b76e404f63e. This closes publication136 only; DATA acceptance does not establish human assembly-swap sign-off, live Inspector behavior, mirror propagation, bench or fit.

**Next planned, HOLD:** Existing F sole coordinator may prepare ticket56 scratch-only per-cell MX-to-RGB-to-Choc-to-restore using existing catalogue normalization/common SetMatrix assembly operations, save/reopen, generic consistency and exports, preserving58 poses/IDs/wiring. Scratch path only: /home/chris/.cache/boardstudio-baselines/sofle-next-unit/assembly-swap56/, from a scratch copy of the accepted archive. No repository, tracked archive, generated fixture, source, test or map edits; no three-project builders or bespoke tests. Luna medium data writer then different Sol6.1 xhigh independent DATA reviewer. Use existing hardware-onboarding escalation for missing components. If a normal operation refuses, stop with a minimal ordinary-operation reproduction and propose a separate shared-bug ticket; no automatic engine fix. This is not human sign-off, live-browser/bulk/propagation proof; browser remains blocked and118 bench/fit stays open. START awaits explicit root grant after tracking commit/index; root owns foreground until then.

### 3. C: take the machine-alone browser timing measurement when scheduled

**Recorded tracker status:** [11](check-speed/issues/11-browser-measure.md) is `ready-for-agent`; its ticket header blocks on completed typed-core [33](typed-core-edits/issues/33-test-speed-rust.md). [12](check-speed/issues/12-browser-quick-wins.md) follows 11 and resolved [34](typed-core-edits/issues/34-test-speed-browser.md); [13](check-speed/issues/13-browser-structural.md) remains `needs-info` after 12. **Root-retained readiness:** C's quiet-machine window is held; no measurement command lease is granted. When root schedules the machine alone, take one bounded11 measurement and reconsider12 from measured evidence. Suggested cohort: C11 measurement, decision on 12, then 12 only if its evidence supports it; 13 remains a decision/research gate, not an assumed implementation.

### 4. A: keep 37 partial and 38 downstream

**Recorded tracker status:** [37](typed-core-edits/issues/37-sleeps-layout-library-ui-page.md) is `implemented (awaiting verification)` and [38](typed-core-edits/issues/38-merge-duplicate-browser-tests.md) is `ready-for-agent`, blocked by 37. **Root-retained readiness:** 37 is partial; its Inspector wait-conversion frontier `web/src/presentation/layout_component_inspector_tests.rs` is deferred, with no ready implementation. The proposed nine tests' 1.08 scheduled seconds do not demonstrate a measured gain. Wait-helper path and Escape/unmount quiescence proof remain unresolved; HIGH shared-helper impacts 29/16 remain. Reopen only on C11 metrics or concrete causal proof plus a new lease. Do not dispatch another automatic audit. After complete 37 acceptance and independent verification, then 38. Suggested cohort: finish/verify 37 only if evidence reopens it; 38 stays out until 37 resolves.

Retained evidence: `/home/chris/.cache/boardstudio-audit/ticket37-followup-20261010.json` and ticket 128's Outcome. Root reports known writers stopped; this queue did not poll them.

### 5. D: identify the exact 35 edit before allocating cleanup

**Recorded tracker status:** [35](typed-core-edits/issues/35-test-maintenance-cuts.md) is `ready-for-agent`, blocked by resolved 34. [24](typed-core-edits/issues/24-test-audit.md) is `claimed`, blocked by resolved 18. **Root-retained readiness:** D is HOLD, with no confirmed assigned worktree, child or running writer; the exact ticket-35 literal-version hunk/path and ready lease are not established. A bounded read audit can identify a concrete slice; 35 is not semantically forced to wait for all of 19. [23](typed-core-edits/issues/23-redo-keyboard-04-on-native-halves.md) gates R1 removal. Producer-dependent transcript/schema/mirror/recipe assertions wait for final producers; root regenerates goldens last. 24 needs fresh verification and final checks. Suggested cohort: scoped D35 audit and only a separately granted exact edit; then eligible cleanup/24 after its remaining dependencies and independent verification. Preserve the ticket headers and do not infer ownership from the `claimed` label.

### 6. Board project sequence and remaining gates

Ticket [25](typed-core-edits/issues/25-shipped-boards-review.md) owns one ordinary saved project per shipped board through existing Core/Archive/Artifact JSON request examples and common operations. The three Sofle variants are Inspector assembly choices in one project. Root-provided readiness: [56](typed-core-edits/issues/56-sofle-sign-off.md) gates one reviewable Sofle artifact plus assembly-swap evidence and human sign-off before remaining boards; it does not wait for all of25 to finish. Tickets [57](typed-core-edits/issues/57-measured-split-hardware-inputs.md) and [79](typed-core-edits/issues/79-unibody-hardware-inputs.md) preserve genuine source-backed hardware/mount/material/process/tolerance decisions and remain gated by56. Ticket [23](typed-core-edits/issues/23-redo-keyboard-04-on-native-halves.md) retains final cross-board review, original archive preservation and integration evidence. Shared mirror/link behavior under19/22 remains required separately; board projects do not wait on19/22/44. Retired per-board tickets are historical only and are not counted as delivery.


## All pending typed-core ticket links

The counts above include every status-bearing open issue. The inventory below groups the exact 34 pending links by stream; links open the ticket headers/body for current status and dependencies.

- Mirror/Core and activation (20): [4](typed-core-edits/issues/04-refresh-included-keyboards.md), [19](typed-core-edits/issues/19-reconcile-mirrored-halves.md), [20](typed-core-edits/issues/20-recipes-build-one-half.md), [21](typed-core-edits/issues/21-mirrored-half-web-adapter.md), [22](typed-core-edits/issues/22-mirrored-half-integrated-verification.md), [23](typed-core-edits/issues/23-redo-keyboard-04-on-native-halves.md), [40](typed-core-edits/issues/40-set-companion-placement-operation.md), [41](typed-core-edits/issues/41-inspector-companion-mode.md), [106](typed-core-edits/issues/106-reconcile-mirrored-matrices-layouts-parts.md), [107](typed-core-edits/issues/107-reconcile-mirrored-hosted-electrical-items.md), [108](typed-core-edits/issues/108-reconcile-mirrored-outlines-assemblies.md), [109](typed-core-edits/issues/109-mirrored-half-exceptions-unlink.md), [110](typed-core-edits/issues/110-inverse-map-mirrored-half-edits.md), [111](typed-core-edits/issues/111-integrate-mirrored-half-reconciliation.md), [112](typed-core-edits/issues/112-inverse-map-hosted-electrical-edits.md), [113](typed-core-edits/issues/113-inverse-map-reviewed-outline-edits.md), [114](typed-core-edits/issues/114-mirrored-edit-compatibility-constraints.md), [115](typed-core-edits/issues/115-prepare-mirrored-edit-transaction.md), [116](typed-core-edits/issues/116-prepare-mirrored-open-repair.md), [117](typed-core-edits/issues/117-prepare-mirrored-preview-history-performance.md).
- Test and maintenance (4): [24](typed-core-edits/issues/24-test-audit.md), [35](typed-core-edits/issues/35-test-maintenance-cuts.md), [37](typed-core-edits/issues/37-sleeps-layout-library-ui-page.md), [38](typed-core-edits/issues/38-merge-duplicate-browser-tests.md).
- Board and hardware planning (10): [25](typed-core-edits/issues/25-shipped-boards-review.md), [42](typed-core-edits/issues/42-global-key-spacing-preference.md), [43](typed-core-edits/issues/43-apply-key-spacing-on-creation.md), [45](typed-core-edits/issues/45-reversible-variant-substitution-decision.md), [46](typed-core-edits/issues/46-sofle-hardware-inputs.md), [56](typed-core-edits/issues/56-sofle-sign-off.md), [57](typed-core-edits/issues/57-measured-split-hardware-inputs.md), [79](typed-core-edits/issues/79-unibody-hardware-inputs.md), [104](typed-core-edits/issues/104-preserve-reviung41-original-archive.md), [118](typed-core-edits/issues/118-sofle-adaptation-hardware-contract.md).

Check-speed's pending links: [11](check-speed/issues/11-browser-measure.md), [12](check-speed/issues/12-browser-quick-wins.md), [13](check-speed/issues/13-browser-structural.md).

## Open but unallocated work and backlog

These items are not counted as pending tickets and do not create dependencies:

- Remaining Inspector visual observation/polish, separately authorized: five objective source/acceptance packets are complete, but no live visual observations have occurred and no rendered CSS defect is established. Ticket 12 only corrected copy; it does not claim visual polish complete.
- Typed Core's [not-yet-specified intent list](typed-core-edits/map.md#not-yet-specified) and the deferred behaviors in the [product backlog](../backlog.md): stale edit previews, Rust convention audit, Undo/Redo, generator error messaging, runtime ownership, geometry ownership, host interfaces, asynchronous cancellation, CAD transport, archive boundaries, identity, outline-edit follow-ups, catalogue boundaries, test-contract preservation, controls and canvas performance. Reproduce behavior before turning an item into implementation work.
- Ticket-level human actions remain those explicitly marked `ready-for-human`, especially 45, 46, 56, 57 and 79; distinguish them from evidence/readiness gaps. The 118 engineering gaps do not imply a new BOM approval step.

## Operating constraints

This queue organizes documented work; it does not activate or revise tickets or dependencies. Keep foreground commands globally sequential. Fresh independent verification, required checks, and the full integration batch remain ticket/batch gates. Root owns integration, maps/tracker, generated goldens and GitNexus refresh. Do not infer worktree ownership or fresh worker status from this snapshot.
