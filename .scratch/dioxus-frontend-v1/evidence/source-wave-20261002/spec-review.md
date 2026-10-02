# Independent source Spec review — frontend wave, 2026-10-02

Reviewed exact implementation commits against integration base `42b9fdefe1941e3441ed7d43d2757e93e3588bde` and the integration worktree's supplied contracts. Read-only source review; no repository files changed and no agents spawned. Existing implementation evidence was inspected, not represented as newly executed verification. Pinned React comparator: `5a472a9426e6e38993361da402cd4ec730feb369`.

## F5.2b — `6fe8fb875585fca7fba4636cf1a9696e55b5406f`

**Changes required; three findings.**

1. **P1 — feedback loses its target identity after settlement.** `web/src/presentation/pcb_wiring/controller.rs:111-123,237-239` stores/returns only key ID and status. Once `pending` is cleared, the effect returns at line 45 and never clears or filters this feedback on subsequent session/document/board/instance changes. Pending feedback likewise remains exposed before an outcome exists. Two physical instances of the same canonical board necessarily reuse key IDs, so the new instance can receive the previous instance's Saved/Failed/Pending feedback. Contract: “feedback is displayed only while document/session/board/instance/key target still matches.” Retain the captured feedback scope and filter it on every projection, independently of settlement.

2. **P2 — the promised consumer mount is absent.** `web/src/presentation/pcb_wiring.rs:159-166,213-236` carries projection/callback props, but neither its props nor `board_wiring` accepts/renders an `Element` slot; the new fields are not consumed by the UI. No feature stylesheet root link was added either. Contract: “F5 exposes its existing Wiring composition position through one private F6-control slot” and “Editor root owns the link.” Supplying data alone does not complete that handoff; coordinate the actual slot/root asset ownership before claiming it delivered.

3. **P1 — required owner-level verification is missing.** The four new tests in `firmware_position_projection.rs` exercise pure projection/`settle_edit`; none invokes the new root callback, Runtime admission, `observe_operation`, or real asynchronous settlement. Existing Core keymap tests cannot prove those paths. Contract explicitly requires Runtime/Core tests for stale admission, exact outcome across accepted advancement, hidden consumer, and persistence failure/fresh retry. Source fixtures also do not exercise the promised auxiliary overlap case. Keep these acceptance gates open.

Projection ordering, legacy-map separation, admission identity checks and unconditional owner placement otherwise follow the contract. RF: no new distinct refactoring takeaway; retain RF-001/RF-006/RF-009, particularly the gap between native pure tests and the actual WASM owner.

## PCB ticket 05 — `0dd1f3ce904dc11ba62d175dba078729d5263472`

**No actionable source Spec finding observed in this bounded proposal-only slice.**

The immutable builder and private packaged adapter implement definition eligibility, conditional existing part overrides, project reversible flag, reversible-only flip recomputation, retained-primary orientation/authored state, mechanical fallback, existing shared construction, new-secondary clearing, and transport-only preservation. The WASM test imports the actual generated module and exercises the production adapter. The native invariant tests and implementation evidence cover the intended boundary; this review reused that evidence without rerunning it.

Ticket 06's live admission, async recheck, exact operation/durability settlement and accepted-only selection are deliberately absent, as authorized by the split. F5.6a's single public paired/history/reopen journey and parent joins remain open. No claim of public UI acceptance is justified by ticket 05 alone.

RF: no new distinct refactoring takeaway observed. Carry RF-006 for canonical-board/physical-instance lineage and RF-009 for actual package-call and paired-evidence accounting; ticket 06 retains RF-001 ownership.

## Keycaps fit — `448c3274693b611ad747d0461de1b18d88245fb8`

**Changes required; three findings plus the existing browser acceptance blocker.** Candidate worktree HEAD `89217f7f93643c0c704b43c800479e9eaf099aea` adds documentation/evidence after the reviewed code commit; the compared delta does not alter this code.

1. **P1 — the request effect subscribes to signals it unconditionally writes.** `web/src/presentation/keycaps_fit.rs:103-122` reads `sequence()`/`state.read()` inside `use_effect` and writes both. Installed Dioxus 0.7.10 `use_effect.rs:17-18` explicitly tracks all callback reads; `use_reactive` does not disable that tracking. Every request schedules another effect, increments the sequence, supersedes the previous request and starts another. This is a concrete source-level loop consistent with the reported CPU-bound editor. Use non-subscribing bookkeeping reads/storage and verify one request per source/retry change, then reproduce the public journey.

2. **P2 — same-source retry/failure can label retained output current.** `keycaps_fit.rs:87-92` defines currentness solely as accepted-source equality. A retry retains an accepted result for that identical source; `begin` marks refreshing and failure sets error, but neither makes `is_current` false. The stale marker at lines 193-195 is consequently omitted, and clean-result copy remains displayed. Contract: “A late or failed reply must never make the old result appear current.” Currentness must also encode successful latest assessment status.

3. **P2 — settled finding grouping/labels do not match the reference.** `keycaps_fit.rs:217-256` emits one article per raw finding and labels a matching key `reference · key`. Pinned React `FindingList.tsx` uses severity-sorted/deduplicated `presentedFindings`, groups by target label, and `findings.ts` labels parts with definition names and supports outline/body targets. Contract requires “groups/labels them with the established finding presentation rules.” Navigation can remain ticket 02; presentation parity cannot.

The recorded CPU hang and missing paired finding/clean/edit/Undo/Redo/reopen plus delayed/error lifecycle evidence remain acceptance blockers. Native resolver tests and successful builds do not waive them. RF: carry RF-003/RF-006/RF-009; no new RF ID proposed. Reactive-loop correction is current correctness work, not deferred refactoring.
