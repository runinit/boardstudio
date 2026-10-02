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
