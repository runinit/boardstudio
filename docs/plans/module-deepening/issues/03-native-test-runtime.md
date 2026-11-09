# 03: Native tests run the real Session and Core

Status: resolved
Type: build
Blocked by: 02, [edit settlement 20](../../edit-settlement/issues/20-preview-only-direct-edit-event.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [02 answer](02-decide-native-test-runtime.md#answer)

## What to build

Replace `runtime_test_stub.rs` with a native test `Runtime` that drives a real
`Session` and `CoreEngine`, and give it and the wasm in-process adapters one gate
vocabulary. Line numbers at `915d305c0`, orientation only.

Blocked by edit settlement 20 because that ticket rewrites the test commit helpers in
`runtime.rs` (`project_name_test_support`) and the direct edit event.

## Interface

- **Gate driver** (`web/crates/runtime/src/`, `cfg(any(test, feature = "test-support"))`):
  one-shot behaviours for the next Core reply and the next save: fail with a reason, or
  hold until released (with an "entered" signal so tests can wait for the call to
  arrive). It is the logic `InProcessCore`'s `OneShotBehavior`
  (`in_process_support.rs:~14`) and `NativeEditDriver`'s `hold_next_*`/`fail_next_*`
  (`edit_ticket.rs:~195`) each implement today.
- **Native `Runtime`** (`lib.rs:36-39` points at it instead of the stub): built from
  `NativeEditDriver`. It keeps the surface native tests use today (`model`, `scope`,
  `operation`, `observe_operation`, `submit`, `electrical_preview_executor_epoch`) and
  the edit ticket port, and adds the gates (`hold_next_core`, `fail_next_core`,
  `hold_next_save`, `fail_next_save`, `release_core`, `release_save`). `submit` drives
  Session effects synchronously through `CoreEngine` and in-memory saves.
- **Kept:** a read-only event log (today's `events`) for assertions on non-edit events.
- **Removed:** `settle` (hand-settling outcomes) and `set_scope`. Tests reach those
  states through the gates and Session (open or close a project).
- **wasm:** `InProcessCore` and `TestPersistence` delegate their one-shot behaviour to
  the gate driver; their public test API (`fail_next_reply`, gates) is unchanged.

## Acceptance criteria

- [x] Natively, a ticket begun on the Runtime settles `Landed`, `Failed` (Core failure,
  save failure) and `Retired` (session change) without the test calling a resolver.
- [x] Gates hold and release the next Core reply and save, so tests observe `Pending`.
- [x] `NativeEditDriver` is gone; the edit ticket's tests use the native Runtime.
- [x] `HookPort` and `PlacementRuntime` (`part_placement.rs:304`, `:2030-2105`) are
  deleted and `part_placement` tests use the Runtime.
- [x] The hand-rolled Session loops in `matrix_transform_lifecycle.rs` (~398),
  `parts/mechanical_profile.rs` (~511-674) and `firmware_position_projection.rs` (~469)
  are deleted; their tests use the Runtime.
- [x] The seven native files that hand-settle and the one that calls `set_scope`
  (`rg -n "\.settle\(|set_scope\(" web`) use gates or Session instead.
- [x] `EditResolver::resolve` hand calls stay for now; they move in 07-09.
- [x] Mounted tests pass unchanged; nothing new compiles into production builds.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
cargo test -p boardstudio-web-layout -p boardstudio-web-parts -p boardstudio-web-pcb --locked
python3 scripts/check.py lint typecheck test
python3 scripts/check.py browser
```

## Outcome

Merged `deepening/03-native-runtime` through `c1e375ae0` after rebase onto
`9b366b160`. Commits add the test-only native Session/Core Runtime, shared one-shot
gates and read-only event log; remove the fake drivers and manual Session loops;
and migrate all named hand-settlement fixtures. Normal native builds have no Runtime,
and WASM production continues to use its existing Runtime.

Real Core execution exposed invalid generated assembly IDs in selected-key placement.
The placement migration includes the necessary sanitized, collision-safe ID fix and
regression coverage. Real Session fixtures also corrected old fabricated Close/Open
expectations: active saves drain before Close/Open, and fixture installation cannot
replace a Session during its pending save. Concurrent scenarios now submit actual
Session events and await observed terminal outcomes.

Lint and typecheck passed; native workspace and footprints tests passed. Affected
browser suites passed: Layout 109 unique tests, Case 60, Parts 50, PCB 38, main page 40,
and Runtime 29. Final parallel Standards and Spec reviews found no remaining issues.
The broad browser attempt stopped at the exact-base reproduced Keymap timeout; its
isolated test passes. The test gate retains the unchanged CAD fixture baseline
(49 passed, 1 failed, 4 ignored; rotated-concave/bottom volume expected 80481.2399,
actual 80579.55733514718, tolerance 0.1).

The Matrix presenter integration test remains unregistered, and PCB physical setup's
legacy test file remains unreachable under existing cfg. Their manual settlement was
migrated, but these dormant scenarios were not executed. The 15 formerly dormant
outline hook scenarios are now enabled and passed against the real WASM Runtime.
