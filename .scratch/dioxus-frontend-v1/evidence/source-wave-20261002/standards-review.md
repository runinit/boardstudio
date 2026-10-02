# Independent Standards review — 2026-10-02

Read-only source review against base `42b9fdefe1941e3441ed7d43d2757e93e3588bde`. Exact comparisons: `git diff 42b9fdef...<commit>`.

| Slice | Reviewed commit | Critical source SHA-256 |
| --- | --- | --- |
| F5.2b | `6fe8fb875585fca7fba4636cf1a9696e55b5406f` | `web/src/presentation/pcb_wiring/controller.rs`: `20fc71c99aa7a8b45d5d3521e4db3a0a3a4ce96a3d07bddb52be8bd3231c1b59` |
| PCB05 | `0dd1f3ce904dc11ba62d175dba078729d5263472` | `web/src/presentation/parts/catalogue.rs`: `d4fc4d05b0a6135ffa2ff23d97213d57697231bf14a2990dab6ece23b9a23c0b` |
| Keycaps fit | `448c3274693b611ad747d0461de1b18d88245fb8` | `web/src/presentation/keycaps_fit.rs`: `678819b48d15987fc172ee098da091d4475cef67f6a738667daf0849865c0c7c` |

Standards: supplied AGENTS.md, CONSTRAINTS.md (Architecture, Rust and Dioxus, Floor, acceptance), CONTEXT.md, ADR 0003, docs/agents/domain.md and issue-tracker.md. Integration has no CONTEXT-MAP.md or .codegraph directory. Dioxus behavior verified against installed **0.7.10** hooks `use_effect.rs`, `use_reactive.rs` and signals `signal.rs:481`; not inferred from React dependencies.

## Findings

- **Keycaps fit — P1, hard correctness/standards violation:** `keycaps_fit.rs:99–113` subscribes the effect to `sequence()` and `state.read()`, then writes both. `use_reactive` adds reactive dependencies; it does not limit tracking to its argument tuple. Dioxus tracks all effect reads. Thus the effect repeatedly invalidates itself, spawns ResolveKeycaps requests, and invalidates prior sequence IDs. Even `source=None` reads/writes sequence. This is a concrete source cause for the activation CPU hang and violates deliberate reactive dependencies/bounded lifecycle rules. Use non-reactive sequence ownership and untracked previous-state reads; prove a mounted hook reaches idle and remains idle after its result.
- **F5.2b — P1, hard correctness/standards violation:** `pcb_wiring/controller.rs:42` passes `(&version,)` where `version` is a Signal handle, without reading its value. Signal equality compares identity. The effect wakes on pending assignment, commonly before the asynchronous operation completes; later outcome changes only an `Rc<RefCell>`, and Runtime version notifications do not rerun it. Saved/failed feedback can remain Pending indefinitely. Subscribe to `version()` and test actual delayed operation settlement. Pure `settle_edit` tests do not exercise this missing lifecycle subscription.
- **PCB05 — P2, explicit exception required:** `main.rs:5` and `presentation/parts/physical_setup.rs:8` add `#[allow(dead_code)]` for future consumers. CONSTRAINTS Floor prohibits new Rust lint allowances without explicit review. Remove them through coherent integration with the consumer or record the exact approved exception; a code comment alone is not approval.
- **PCB05 — P2, approval evidence required:** `catalogue.rs:237,252` widens existing private `load_ergogen_module` and `call_catalogue` to `pub(super)`. CONSTRAINTS/AGENTS require explicit permission for member visibility widening. The reviewed packet authorizes a private proposal callable, but inspected evidence does not establish approval for these existing-member changes. Preserve private placement or supply exact authorization.

Immutable accepted snapshots, Core-owned edits, and proposal nonmutation are otherwise preserved in inspected changes. No new public wire schema or engine/CAD member export was observed. No additional smell-only finding is raised.

## Verification and acceptance limits

Executed all three exact-commit `git diff --check` comparisons: passed. Inspected implementation evidence and native/WASM test paths; did not rerun builds or browser journeys during this review. Keycaps implementation adds no lifecycle test, and its reported native `--lib` tests exclude the wasm-gated presentation hook. Its report correctly leaves browser activation and paired acceptance blocked. F5.2b reports four pure projection/settlement tests, which cannot validate notification wiring. PCB05 reports real packaged WASM normalization tests plus native immutability/invariant tests; dependent Editor owner and paired acceptance remain open. No complete-slice acceptance is granted.

## Required RF handoff

Coordinator must retain these observations under existing RF IDs: **RF-001** root-owned operation settlement/reactive lifecycle; **RF-003** immutable prepared Case artifact lifetime; **RF-006** board/instance lineage; **RF-009** pure native tests versus mounted WASM lifecycle/parity coverage. PCB05 already supplies RF-006/RF-009 and Keycaps supplies RF-003/RF-006/RF-009 in their implementation handoffs. F5.2b explicitly asks the coordinator to classify its native-versus-WASM seam under RF-009. New concrete lifecycle findings must be repaired now, not deferred as refactoring. No new RF ID proposed. Shared RF ledgers were not edited by this read-only reviewer.

## F5.2b corrected-source re-review — `ebe58a38b250e6ba35b683aa7f03155a3e8046a4`

Reviewed 2026-10-02 in `/home/chris/.local/share/boardstudio/worktrees/f5-firmware-position-handoff-20261002`. Worktree HEAD was merge `22c154bf4bb3aaf9f9df07c3c60b126862cc1474`; `git diff ebe58a38..HEAD -- web/src` was empty, so exercised source matches the reviewed implementation.

Exact source SHA-256:

- `web/src/presentation/pcb_wiring/controller.rs`: `5d28a768cd7e7990a5e092ca262d6ca88a429139d814a3e2e996d27aa7fa1998`
- `web/src/firmware_position_projection.rs`: `ae76ceb45e922b0c53dc942461107b4a84fa92e3e091f7c4a05b2641d96b840e`
- `web/src/presentation/pcb_wiring.rs`: `90a0f4846a153452f0b463010631c723bd516b71d49819dad9bf06fb40080c02`

**Original F5 P1 resolved by source inspection.** `observed_version = version()` at controller.rs:41 is the numeric `use_reactive` dependency. Runtime notification changes now re-run outcome settlement. The tracked pending read schedules one final pass when pending becomes None; that pass exits without writing, so this is bounded rather than the Keycaps self-trigger loop. Feedback writes do not read/subscribe to feedback inside the effect.

Feedback now includes session/document/board/instance Scope, scope generation and key identity, and visibility checks require the current projected key. Plan tokens/revisions are deliberately excluded from the stable feedback target, allowing refreshed accepted plans to retain feedback while moved targets hide it. The new private `firmware_controls: Element` propagates through Workspace composition and is actually rendered in board wiring. Root supplies an empty element pending F6's consumer, as appropriate for this handoff.

The added native test uses real `Session`, `CoreEngine`, and the actual `OperationOutcomes` observer. It verifies scoped admission and delayed durable acknowledgment; Core success alone leaves the observer unset. It does not mount `use_firmware_position_edits` or exercise browser callbacks, so it does not prove reactive wiring at runtime.

**Remaining P2 Standards blocker:** `firmware_position_projection.rs:87` adds `#[allow(clippy::too_many_arguments)]` above the new ten-argument admission helper. CONSTRAINTS Floor prohibits new Rust lint allowances without an explicitly reviewed exception. No such exception was supplied. Bundle the immutable admission inputs into a private struct instead; do not silently waive the rule. Original asynchronous browser/hook regression evidence remains a separate unperformed gate, not covered by the six passing pure/session tests.

Executed `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page firmware_position_projection -- --nocapture`: **6 passed**. Exact correction `git diff --check ebe58a38^ ebe58a38`: **passed**. No source edited.

Disposition: original lifecycle P1 repaired; source integration awaits removal/approval of the new allowance. F5.2b/F6 ticket completion remains distinct and must retain mounted delayed outcome and edit/save/Undo/Redo/reopen browser acceptance. RF-001/RF-009 observations still apply; no new RF ID.

## PCB05 corrected-source acknowledgment — `9e9459a5826514d7cb060cac7a073cafcb834582`

Reviewed exact clean HEAD in `/home/chris/.local/share/boardstudio/worktrees/f56-physical-proposal-ticket05-20261002` on 2026-10-02. **Earlier PCB05 Standards findings resolved; clear for source integration as a tested prerequisite, not production-mounted completion.**

Both `#[allow(dead_code)]` attributes were removed. Catalogue `load_ergogen_module`, `call_catalogue`, imported-module loader and normalization implementation are private. The new test adapter receives a supplied module and delegates through catalogue-owned code; it does not export the private JS loader or library API. Pure proposal semantics in `web/src/physical_setup.rs` are unchanged, including immutable input and single proposal ownership.

The prerequisite module is now `cfg(all(feature="page", test))`; presentation adapter and catalogue proposal callable are `cfg(all(test, target_arch="wasm32"))`. Therefore this commit proves the proposed callable through native/WASM tests but exposes no production Runtime entrypoint. Ticket06 must activate those modules with the first Editor-owned consumer, supply the production packaged-module loading path, and recheck admission/current-source handling. The revised implementation record explicitly states that limit. Do not use a successful production build as evidence the test-only proposal path is mounted, or mark the full callable/parent acceptance complete prematurely.

Preserved package-test proof: `pnpm run test:web:physical-setup` rebuilt packaged generator assets and ran the actual `reversible_proposal_uses_the_packaged_gateron_normalizer` WASM test under Node: **1 passed**. This exercises test adapter → catalogue preparation → actual packaged Gateron normalizer, preserving source document, eligible part override and solder/hotswap assertions. Native `cargo test --locked --manifest-path web/Cargo.toml --features page physical_setup -- --nocapture`: **7 passed**. `git diff --check 0dd1f3ce..9e9459a5`: **passed**.

Source SHA-256: `main.rs` `79cb1f34a4314474bc9ba7a698affca643a5cbe0179446e497fd04865905f428`; `presentation/parts/catalogue.rs` `18b6d599e916569f890c4aa232a9e41b1125783d3f1044acf39e1d8b6bce0716`; `presentation/parts/physical_setup.rs` `f54c4e2464c21fb25fb859128c8e4507f1506c151afb4847c344bac041303573`; `physical_setup.rs` `205776a07f91c93d465253698ac544977e8cf458e1ed82a4c53212cd4c89081c` (all relative to `web/src`). RF-006/RF-009 handoff retained; no new RF ID.

## F5.2b final mechanical acknowledgment — `05b4611b8a8adcdd02f89a53a0efab70bfab5394`

Reviewed exact clean HEAD in the F5 worktree on 2026-10-02. **Clear for source integration; full ticket/browser acceptance remains open.** The `too_many_arguments` allowance is removed and replaced with the private, borrowed `FirmwarePositionAdmission` input struct. All ten prior admission values map directly into the same comparisons; no scope, revision, token, executor, real-key or workspace guard was removed. Production callback and real Session/Core test both call that same helper. No new suppression, visibility widening, or behavioral drift occurs in this mechanical correction. Numeric Runtime version subscription, bounded pending reset, scope-qualified feedback, and rendered private Element slot remain as reviewed above.

Executed focused `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page firmware_position_projection -- --nocapture`: **6 passed**; `git diff --check ebe58a38..05b4611b`: **passed**. Source SHA-256: `firmware_position_projection.rs` `3210f494edde856e4265e65938c8c7a31fe21cf18f01365aad3cb86a9d334ec6`; `presentation/pcb_wiring/controller.rs` `48258f7f15cb7cc0ad656a47d311f7473b38eac8d58d7eb3ca6d4aefb47b3136`.

This clears the earlier F5 source findings, not the outstanding mounted async operation/browser proof. F6 consumer integration must still exercise delayed saved/failed settlement, target changes, controls, Undo/Redo and saved reopen. RF-001/RF-009 and earlier verification limitations remain; no new RF ID.

## Keycaps corrected-source re-review — `4d9196d75e272addced891b6988a9dbd330064e4`

Reviewed exact clean HEAD on 2026-10-02 in `/home/chris/.local/share/boardstudio/worktrees/m1-keycaps-fit-findings-20261002`. Includes original `448c3274693b611ad747d0461de1b18d88245fb8`, loop repair `1741404b9a1da02de53dc5523524525324716db3`, and currentness/presentation repair `4d9196d7`. No candidate source/build edited during review.

Source SHA-256: `web/src/presentation/keycaps_fit.rs` `11f28a06bb8bf233ebec80b3ef19d212a8fd4ce2d97096c20e965f53336a8b39`; `web/tests/keycaps_fit_lifecycle.rs` `5e3cc6e8b20001df99e8994c2456fffba856e91114c799c4b39a9596d6e234e6`.

### Resolved behavior

The original P1 effect loop is repaired: sequence/previous-state reads use `peek`, and completion performs one state write without duplicate set. The mounted native test imports the production hook and uses Dioxus VirtualDom scheduling. Author's retained red evidence reports absent-source **17 renders / 0 requests** and accepted-source **17 renders / 16 requests**, both failing to settle; loop repair produces two passes with exactly one accepted-source request.

`is_current` now requires no refresh/error plus exact source equality. A failed or pending same-source retry keeps the prior accepted result stale; only successful matching-revision finish replaces accepted data. Grouping/deduplication, stable severity ordering, target-label precedence and fitted-outline guidance were compared to actual `app/src/ui/findings.ts` / `FindingList.tsx`. Navigation remains a downstream issue. Author reports a red mutation restoring old currentness failed the pending assertion and a raw-findings mutation failed severity/group order, both exit101, followed by restoration. These red records were reported by the author, not rerun by this reviewer during the candidate build.

### Unresolved Standards findings

1. **P2 — detached async completion can access disposed signals.** `keycaps_fit.rs:121–133` uses `wasm_bindgen_futures::spawn_local`; unlike Dioxus `spawn`, it survives Editor unmount. After await, `sequence.peek()` and `state.write()` have no liveness guard. `presentation.rs:386` removes Editor when accepted state disappears, so closing/reopening while resolution is pending can produce a disposed-signal panic on a late result. Installed dioxus-signals 0.7.10 `ReadableExt::peek_unchecked` unwraps the borrow result. Add a scope-owned alive flag/drop guard checked before any post-await Signal access, or an equivalently verified cancellation lifecycle. The native harness substitutes Dioxus `spawn`, which cancels on scope drop and therefore masks this browser lifetime; add a delayed completion after unmount test with equivalent detached execution.
2. **P2 — retry handler allocates per Editor render.** `keycaps_fit.rs:138` calls `EventHandler::new` in the hook body. Installed dioxus-core 0.7.10 `events.rs:484–497` explicitly warns that these callbacks persist until scope disposal; its owner inserts each allocation. `use_callback` retains one handle and replaces its closure. Use it here to bound the Editor-lifetime allocation; this is source-backed, not a claimed measured performance regression.
3. **P2 — unapproved test suppression/member widening.** `web/tests/keycaps_fit_lifecycle.rs:45` adds `#[allow(dead_code)]`; `KeycapsFitState::{begin,finish,is_current}` change private→`pub(super)` for external tests. CONSTRAINTS Floor and member-visibility rules require explicit approval, which was not supplied. Keep pure tests in a child module with private access, and exercise the UI/seam without the allowance. Test-only motivation does not by itself waive either rule.

Executed exact `git diff --check 448c3274..4d9196d7`: passed. No tests rerun concurrently with the production candidate build. Source integration is **not yet clear** at this revision; original loop P1 is resolved, but these bounded lifecycle/policy repairs remain. Browser finding/edit/Undo/Redo/reopen and delayed-failure acceptance stay open independently.

RF handoff: add concrete callback/lifetime evidence to existing RF-001 and test-executor lifetime mismatch to RF-009; retain RF-003/006 prepared-artifact/physical-source observations. No new RF ID.
