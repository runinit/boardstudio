# 03: Native tests run the real Session and Core

Status: claimed
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

- [ ] Natively, a ticket begun on the Runtime settles `Landed`, `Failed` (Core failure,
  save failure) and `Retired` (session change) without the test calling a resolver.
- [ ] Gates hold and release the next Core reply and save, so tests observe `Pending`.
- [ ] `NativeEditDriver` is gone; the edit ticket's tests use the native Runtime.
- [ ] `HookPort` and `PlacementRuntime` (`part_placement.rs:304`, `:2030-2105`) are
  deleted and `part_placement` tests use the Runtime.
- [ ] The hand-rolled Session loops in `matrix_transform_lifecycle.rs` (~398),
  `parts/mechanical_profile.rs` (~511-674) and `firmware_position_projection.rs` (~469)
  are deleted; their tests use the Runtime.
- [ ] The seven native files that hand-settle and the one that calls `set_scope`
  (`rg -n "\.settle\(|set_scope\(" web`) use gates or Session instead.
- [ ] `EditResolver::resolve` hand calls stay for now; they move in 07-09.
- [ ] Mounted tests pass unchanged; nothing new compiles into production builds.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
cargo test -p boardstudio-web-layout -p boardstudio-web-parts -p boardstudio-web-pcb --locked
python3 scripts/check.py lint typecheck test
python3 scripts/check.py browser
```
