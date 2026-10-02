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

## F5.2b corrected-source re-review — `ebe58a38b250e6ba35b683aa7f03155a3e8046a4`

**Source-integration clearance: clear for the private handoff, with no new actionable source Spec finding observed. Full F5.2b/F5 acceptance: still open.**

Reviewed the exact delta from `6fe8fb875585fca7fba4636cf1a9696e55b5406f` to `ebe58a38b250e6ba35b683aa7f03155a3e8046a4`. Worktree HEAD is now merge `22c154bf4bb3aaf9f9df07c3c60b126862cc1474`; its delta from the reviewed source commit changes documentation/evidence only, so the tested source remains identical.

- **Feedback target correction verified.** `FirmwarePositionFeedbackTarget` retains the complete UI Scope, scope generation and key ID. Editor composition filters feedback against current scope/generation/projected key membership every render. Saved/Failed/Pending feedback can no longer be blindly forwarded into another board, physical instance or session. Stable feedback identity excludes admission token/revision, allowing a matching target to retain settlement across accepted plan refresh. The new visibility regression covers refreshed identity, other board/instance/generation and missing key.
- **Actual Element slot verified.** `PcbWiringInspectorProps::firmware_controls` is an `Element`, forwarded by workspace composition and rendered inside board Wiring between resolver controls and plan details. Root supplies an empty element until the F6-owned initial consumer mounts. This is now a real composition seam, not unused data props. F6 controls and its separately owned stylesheet/root link are not implemented or accepted by this commit; they remain initial-consumer integration work.
- **Owner-path evidence materially improved, but not complete.** The new test opens a real `Session` through `CoreEngine` and persistence, uses the production admission helper, observes an exact `OperationOutcomes` slot before submitting `SetKeyBinding`, drives Core and save completion, and asserts the accepted binding/revision and exact terminal outcome. It also rejects stale board and executor inputs. This is valid Session/Core evidence, not a mounted Runtime/Dioxus test. Failure/fresh retry, hidden/unmounted consumer settlement, event-time stale callback admission and delayed asynchronous effect execution remain mandatory mounted regressions.
- **Numeric version dependency verified.** The observer reads `let observed_version = version()` and passes that numeric value into `use_reactive`; it no longer supplies a stable Signal handle as the external dependency.

Executed: `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page firmware_position_projection -- --nocapture` — all six targeted tests passed. No WASM/browser/build checks were rerun in this review. Existing row-order/auxiliary-overlap and full handoff acceptance requirements are not waived.

Retain the mounted async browser regression as a mandatory F6 initial-consumer gate before claiming the handoff complete, including target changes, hide/unmount, accepted revision/plan advancement, persistence failure/new retry, and truthful accepted-value display. Paired controls/edit/Undo/Redo/reopen and F5.2/F8.2 joins remain open. RF: no new distinct takeaway; retain RF-001/RF-006/RF-009 and the real Session-versus-mounted-owner verification distinction.

## PCB05 corrected-source re-review — `9e9459a5826514d7cb060cac7a073cafcb834582`

**Source-integration clearance: clear as an explicitly test-scoped prerequisite. Production accepted-callable gate: not yet satisfied.** Exact worktree HEAD matches this commit and is clean. No actionable new proposal-rule finding observed.

Existing catalogue loader/import/catalogue/normalization helpers return to their original private visibility. The correction removes dead-code allowances. `main.rs` registers the native proposal module only under `feature=page,test`; `parts.rs` and the new catalogue proposal adapter additionally require WASM tests. Consequently a normal page binary has no callable proposal builder at this commit. Ticket05 must not be marked fully accepted, nor may its ticket06 dependency be reported as an already-live production port.

The packaged WASM test does invoke the real generated module, then the parts proposal adapter, catalogue-owned proposal dispatcher and common `physical_setup::propose` rules. Eligible definitions use the same `construction_definition_with_support` function as production catalogue normalization. This is real package execution with shared implementation, but the proposal dispatcher itself is currently absent from production. Its test does not establish Runtime ownership, production resource-URL loading or an Editor-mounted consumer.

Ticket06 can legitimately activate this source without widening existing members: compile the same modules/functions for the actual WASM page consumer; keep package acquisition and proposal preparation within catalogue ownership; let a Parts-owned bridge/controller consume the existing `pub(super)` proposal seam and expose only the new, purpose-specific Editor mount/intent seam allocated by the coordinator. Existing private loader/normalizer functions need not become visible to siblings or root. Do not duplicate the tested proposal/normalization logic in that activation. Production async acquisition must be followed by the full ticket06 source recheck before submission, with exact operation, durability and accepted-selection verification.

Executed targeted native check: `cargo test --locked --manifest-path web/Cargo.toml --features page physical_setup -- --nocapture` — seven proposal tests passed. The recorded real-module WASM and strict-check evidence was inspected, not rerun here. Re-run the shared packaged test and production WASM checks when ticket06 activates the consumer. F5.6a paired controls/history/reopen and all parent joins remain mandatory. No new refactoring takeaway; carry RF-006/RF-009 and ticket06's RF-001 owner boundary.

## F5.2b final narrow acknowledgment — `05b4611b8a8adcdd02f89a53a0efab70bfab5394`

**Source-integration Spec clearance retained; no new actionable finding.** Clean worktree HEAD matches this exact commit. Compared with previously cleared `ebe58a38b250e6ba35b683aa7f03155a3e8046a4`, the executable change replaces the many-argument admission call and its Clippy allowance with private `FirmwarePositionAdmission` fields. Every prior identity predicate and production call-site value is preserved; the tests use the same struct. Documentation-only intervening merge does not alter the reviewed source behavior.

Exact current file SHA-256:

- `web/src/firmware_position_projection.rs`: `3210f494edde856e4265e65938c8c7a31fe21cf18f01365aad3cb86a9d334ec6`
- `web/src/presentation/pcb_wiring/controller.rs`: `48258f7f15cb7cc0ad656a47d311f7473b38eac8d58d7eb3ca6d4aefb47b3136`

Reran the same targeted native firmware-position command after this correction: all six tests pass. This source may be integrated and released to the F6 initial consumer. The previous acknowledgment's full-ticket limits remain unchanged: production F6 control/stylesheet mounting and mandatory mounted async Runtime/Dioxus regression, paired edit/Undo/Redo/reopen and F5.2/F8.2 joins are still open. No new refactoring takeaway; no gate is waived.

## Keycaps final corrected-source acknowledgment — `13533ccf373cbdda0ca0c478744907117ea906cf`

**Source-integration Spec clear; all three original source findings are resolved. Full paired acceptance remains open.** Reviewed final source at merged HEAD `fa269477a181573df5f2d21a008fd9f327e4a254`; the merge does not change the reviewed fit module, lifecycle tests, Runtime request or Keycaps workspace files. Uncommitted report/screenshot updates were inspected only as evidence, not mistaken for source changes.

Exact hashes verified:

- `web/src/presentation/keycaps_fit.rs`: `13c5644743c21a0701378a68d166b1806b02484923899ae91182de6be04e4c65`
- `web/tests/keycaps_fit_lifecycle.rs`: `8d8e67db790aa5d03c667d4703a1b47657975fa8903d4f89c9be555ed94c72c0`

The request effect now uses non-subscribing `peek` reads for mutable bookkeeping and preserves one request per source/retry change. Same-source pending/retry/failure is non-current because `is_current` requires a settled success without error; retained clean output is explicitly described as an earlier assessment. Findings follow the pinned React oracle's feature-wrapper deduplication, merged target IDs, stable error/warning/info sorting, first-seen target groups, outline/part+definition/matrix/body/board label precedence, fallback scope label and fitted-outline note. Keycaps' reference `FindingList` receives no mechanical assembly, so omitting an assembly-layer label branch here is correct. Finding navigation remains separate ticket02 scope.

The final follow-up adds an Rc/Cell mount guard checked before any signal access after the detached await and uses stable `use_callback` retry ownership. The harness imports the production hook/component directly and models genuinely detached pending tasks, then drops the VirtualDom before settling the request. This exercises the prior disposed-signal failure rather than merely cancelling the test task with the component. Six tests are present: four mounted lifecycle scenarios and two source state/oracle tests; this is not six browser-worker integration tests. The supplied independently passing test evidence and documented red/green regressions were reused; no redundant build/test run was added while the candidate build is active.

No new actionable source Spec finding observed. The fixed `4d9196d` activation screenshot/run supports the loop fix but predates the final unmount correction. Require the final source-stamped candidate and full paired finding/clean/edit/Undo/Redo/save-reopen journey, plus delayed supersession/failure and responsive Inspector evidence, before accepting ticket01/F6C.4/INT.2. No gate waived; RF-003/RF-006/RF-009 carry forward, no new RF ID.
