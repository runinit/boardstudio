# 06: Core applies mechanical settings changes

Status: ready-for-agent
Type: build
Blocked by: 05
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [02](02-mechanical-settings-patch.md)

## Question / goal

`mechanical::edit::apply` performs the settings families of the moved change
vocabulary against the accepted document, using `effective_inputs` to resolve the
target and Core-owned defaults. It validates, persists to the canonical or instance
configuration, and returns only the durable ids whose values actually changed. The
Case controller keeps its ports, drafts and `flush_prepared` ordering but submits a
`MechanicalEdit` instead of mutating a cloned document.

## Seams under test

- `boardstudio_core::mechanical::edit::apply` through `CoreEngine` `Edit` commits.
- `boardstudio_core::mechanical::evaluate_edit` (Changed/Unchanged/Retire).
- The Case controller's existing native/WASM tests.

## TDD cases

1. Each settings family (enable/disable, method/mount, dimensions, battery, closure
   hardware, critical fits, hardware, mount collections, processes, profiles,
   stabilizers) applies through the operation and reports the changed configuration
   owner.
2. A change equal to the accepted value returns `Unchanged` and adds no history step.
3. A malformed value returns a Core error and mutates nothing.
4. Explicit empty mount collections survive; absent ones stay absent.
5. A vanished or ineligible target retires; a wrong-board profile target errors.
6. Two queued edits to one field keep both accepted edits and one report each.

## Verification

`cargo test -p boardstudio_core --locked`, `cargo test -p boardstudio-web-case
--locked`, `wasm-pack test --headless --chrome web/crates/case --locked --lib`.
