# 01: Runtime Core and persistence ports with an in-process test adapter

Status: claimed
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

This is a prefactor with no user-visible change. Runtime currently talks to a
concrete Core worker and to browser storage, and tests patch feature-specific
branches into Runtime to avoid them. Put Core execution and document saving behind
two narrow ports. Production adapters wrap the existing Core worker and browser
store. A test adapter runs the real `CoreEngine` in-process and keeps saves in
memory, with gates that can hold or fail one Core reply or one save.

When this is done, any mounted browser test can say "install the in-process
adapter on this Runtime". Every Session effect, and every direct Core request
Runtime makes (resolvers, archives, artifacts), then runs through the real engine,
with no feature-named test branches.

## Background you need

- Read `docs/architecture.md`, `AGENTS.md`, and the "Implementation Decisions"
  section of the [spec](../spec.md).
- Runtime keeps `core: RefCell<Rc<CoreWorker>>` and calls `request`, `archive`,
  `artifact`, `ready` and `close` on it. It compares worker identity with
  `Rc::ptr_eq` to drop replies from a replaced worker. The Core-effect branch
  restarts the worker by constructing a new `CoreWorker` from a resource URL.
- A narrow executor trait already exists for firmware export
  (`FirmwareExportExecutor`: `request` + `archive`, boxed futures), implemented for
  `CoreWorker`. Use it as the model for the new Core port.
- The in-process pieces already exist:
  - `CoreEngine::handle` serves Core requests;
  - the Core worker serves archives with `boardstudio_core::archive::request` and
    artifacts with `boardstudio_core::artifact_request`.
  An in-process adapter can call these synchronously.
- Existing test-only fields in Runtime show what to replace:
  - `project_name_test_core` (an in-process `CoreEngine` used only by project-name
    tests);
  - `project_name_persist_test_behavior` (fail or gate the next save);
  - the `project_name_test_support` module (install / gate / fail / observe / run
    pending effects).
  Generalise these. Don't add a parallel mechanism.
- Starting points (orientation only, taken at `9548275`):
  - `web/crates/runtime/src/runtime.rs` (struct ~427, `run` ~2138, firmware executor
    trait ~266, project-name support ~7738);
  - `web/crates/host/src/host/core_client.rs` (`CoreWorker`);
  - `web/src/core_worker.rs` (how the worker serves archive/artifact).

## Approach

1. Define a Core executor port (one trait) whose methods cover what Runtime uses:
   request, archive, artifact, ready and close. Implement it for `CoreWorker`.
   Store the executor in Runtime as `Rc<dyn …>` so `Rc::ptr_eq` identity checks
   keep working.
2. Give Runtime a factory for new executors so that `RestartCoreExecutor` creates
   a fresh executor of the same kind (a worker in production, a fresh in-process
   engine in tests).
3. Define a document persistence port with only the save Runtime performs for
   Session `Persist` effects. Implement it for the browser store. Loading, listing,
   deleting and active-project preferences stay on the browser store; don't widen
   this port.
4. Add a test-only in-process adapter for both ports:
   - Core requests are served by `CoreEngine::handle`, archives and artifacts by
     the Core crate functions the worker uses;
   - saves go to memory;
   - you can gate the next Core reply or save (await a release) or fail it with a
     reason.
   Gating the next save already exists; add Core-reply gating.
5. Move project-name tests onto the adapter, then delete `project_name_test_core`,
   `project_name_persist_test_behavior` and the matching branches in `submit` and
   `run`. Keep the support module's public helper names working, or update their
   callers in the same change.
6. If the firmware executor trait becomes a strict subset of the new port, you may
   fold it in. If that grows the diff beyond this ticket, leave it and note a
   follow-up in your Outcome.

## Acceptance criteria

- [ ] Runtime has no `#[cfg(test)]` fields or branches for project-name Core or persistence behaviour.
- [ ] In production, Runtime executes Core requests and saves exactly as before: same worker URL, restart behaviour, reply identity checks and asset filtering on save.
- [ ] A browser test can install the in-process adapter, open a document, submit an `Event::Edit`, and observe the accepted document change and a saved copy in memory.
- [ ] A browser test can gate a save, observe `Durability::Saving`, release it and observe `Saved`; and fail a save and observe `RecoveryRequired`.
- [ ] A browser test can gate a Core reply and observe the edit still pending until release.
- [ ] Executor restart under the in-process adapter yields a fresh engine, and replies from the old one are ignored.
- [ ] All existing project-name and firmware-export browser tests pass unchanged in behaviour.

## Verification

Use the README commands. At minimum:

```sh
python3 scripts/check.py --list            # see step names
python3 scripts/check.py typecheck test     # wasm typecheck + native tests
wasm-pack test --headless --chrome web/crates/runtime --locked --lib
```

Also run the browser tests for any file whose tests you touched, plus the
`browser` step's inventory check (`python3 scripts/check-wasm-tests.py`). New
browser tests must be registered the way the browser runner expects; follow the
README and the existing test-owner files under `scripts/`.

## Out of scope

- CAD workers, export workers, keycap/case previews and generation jobs (only Core
  and save go behind ports).
- Layout-inspector and definition-name interception modes (tickets 05 and 07).
- Any change to Session or Core.

## Pitfalls

- Runtime compiles only for `wasm32` with the `page` feature; native
  `cargo test` doesn't compile it. Always run the wasm typecheck and the browser
  tests.
- Don't change the worker request-ID format (`m1-<n>`). Session checks it.
- Preserve the order of effects in `run`: the persistence branch must still remove
  committed asset bytes only after a successful save.
