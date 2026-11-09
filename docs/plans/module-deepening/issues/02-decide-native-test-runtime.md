# 02: Decide the native test Runtime's shape

Status: resolved
Type: grilling
Blocked by: —
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Review: [candidate 02](../../../investigations/architecture-review-2026-10-07.html#c2)

## Question

How do native panel tests reach the real Session and Core through the same Runtime
surface panels use in the browser?

Line numbers are at `915d305c0`, orientation only.

## Evidence

- `web/crates/runtime/src/lib.rs:36-39` replaces the `runtime` module natively with
  `runtime_test_stub.rs` (65 lines): it records events and never resolves.
- `edit_ticket.rs` tests hold `NativeEditDriver`, a real `Session` plus `CoreEngine`
  with in-memory saves and hold/fail gates for the next Core reply or save. It is
  private to that test module.
- `web/crates/layout/src/part_placement.rs:304` `PlacementRuntime` and `HookPort`
  (`:2030-2105`) re-encode Session resolution: base revision, `test-edit-{id}`
  transactions, Unchanged→Completed, Retire→Rejected.
- Hand-rolled Session loops: `matrix_transform_lifecycle.rs:~398`,
  `parts/mechanical_profile.rs:~511-674`, `firmware_position_projection.rs:~469`.
- `in_process_support.rs` (`InProcessCore`, `TestPersistence`) is the wasm adapter set
  with the same gate vocabulary; it depends on `BrowserStore` and the browser Runtime.
- Native vs wasm test counts: layout 66/93, case 17/60, keymap 2/21, library 0/13.

## Options

- **A. Native `Runtime` built from `NativeEditDriver`**: same public surface the stub
  has today, plus `drive`, `hold_next_core`, `fail_next_save`, `release_*`. Small;
  browser-only effects (renderer, CAD, workers) stay unavailable natively.
- **B. Compile the real `Runtime` natively** by moving its remaining browser handles
  behind host ports with native adapters. Deepest, but large: `runtime.rs` holds
  renderer, CAD and worker state.
- **C. A shared driver crate** used by both the native Runtime and `InProcessAdapters`
  so the gate vocabulary has one implementation.

## Decide

1. Option, and which Runtime methods the native surface must answer.
2. Whether the gate API is shared with `InProcessAdapters` (one vocabulary) or only
   mirrored.
3. Feature gating: `test-support` only, never in production builds.
4. Which hand-rolled loops and `HookPort`/`PlacementRuntime` are deleted in ticket 03
   versus left for the panel migrations (07-09).

## Done when

`## Answer` records each decision and ticket 03's interface section is filled in.

## Answer

Decided with the user on 2026-10-07.

1. **Option C.** The native `Runtime` (replacing `runtime_test_stub.rs`) drives a real
   `Session` and `CoreEngine` with in-memory saves, built from `NativeEditDriver`.
   Renderer, CAD and worker effects stay unavailable natively.
2. **One gate vocabulary.** A shared gate driver (hold or fail the next Core reply or
   save, then release) behind `cfg(any(test, feature = "test-support"))` in
   `web/crates/runtime`. Ticket 03 converts both the native Runtime and the wasm
   `InProcessCore`/`TestPersistence` to it; the wasm adapters' public test API
   (`fail_next_reply`, gates) is unchanged.
3. **Test-support only;** nothing new in production builds.
4. **Surface.** The native Runtime keeps a read-only event log for assertions on
   non-edit events. Hand-settling (`settle`) goes: tests use the gates. Tests that
   change scope do it through Session (open or close a project), not `set_scope`.
5. **Deletions in ticket 03:** `HookPort`, `PlacementRuntime` and the three hand-rolled
   Session loops (`matrix_transform_lifecycle.rs`, `parts/mechanical_profile.rs`,
   `firmware_position_projection.rs`). Each panel's `EditResolver::resolve` hand calls
   move in that panel's migration ticket (07-09).
