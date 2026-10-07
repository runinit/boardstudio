# 01: Runtime Core and persistence ports with an in-process test adapter

Status: resolved
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

## Outcome

Commits: aa67565bc (claim), a62b6a530 (ports + adapter), a3c1d00d2 (review fixes).

- **Core executor port.** `CoreExecutor` (request, archive, artifact, ready, close) lives in
  `web/crates/host/src/host/core_client.rs` beside `CoreWorker` and is implemented by it via
  delegation; Runtime stores `Rc<dyn CoreExecutor>` so `Rc::ptr_eq` and
  `core_executor_identity` capture checks keep working. `RestartCoreExecutor` builds the
  replacement through an injected factory (production: same worker URL and error strings).
  The firmware export executor trait was folded in (step 6): firmware paths use
  `Rc<dyn CoreExecutor>`, and the test `ControlledExecutor` implements the full port.
- **Persistence port.** `DocumentPersistence` (runtime.rs) covers only the save a Session
  `Persist` effect performs; implemented for `BrowserStore`. Loading, listing, deleting and
  the active-project preference stay on `BrowserStore`. The Persist branch keeps asset
  filtering before the save and committed-asset removal after success.
- **In-process test adapter** (`in_process_support.rs`, `test-support`-gated): `InProcessCore`
  serves requests via `CoreEngine::handle` and archives/artifacts via the Core crate
  functions the worker uses, mirroring the worker's reply-id checks; `TestPersistence` saves
  through the production store port by default or into memory (documents + assets). Either
  port accepts a one-shot gate that holds or fails the next reply/save.
- **Project-name test support migrated.** `project_name_test_core`,
  `project_name_persist_test_behavior` and their branches in `submit`/`run` are gone;
  `project_name_test_effects` became a general `held_effects`. The support module's helper
  names still work (`install` routes Core through the adapter and attaches the gated
  persistence port; `fail_next_persist`/`gate_next_persist` unchanged). New helpers:
  `install_memory_persistence`, `gate_next_core_reply`, `fail_next_core_reply`,
  `saved_document`, `in_process_core`.
- **Interpretation note.** `install` keeps saving through the production browser-store port
  (behind the gate decorator that replaced `project_name_persist_test_behavior`) so the
  library/pcb/runtime tests that assert durable browser storage and the active-project
  preference pass unchanged in behaviour; memory saves are opt-in via
  `install_memory_persistence`. The new acceptance tests use the opt-in and cover every
  memory-saves criterion.

Checks (all pass): `python3 scripts/check.py typecheck`; `wasm-pack test --headless
--chrome web/crates/runtime --locked --lib` — 33 passed (28 existing + 5 new adapter tests:
edit accepted + saved copy in memory; gate save → `Durability::Saving` → released → `Saved`;
fail save → `PersistenceFailed` + `RecoveryRequired` with accepted value kept; gate Core
reply → edit pending until release; restart → fresh engine, old executor closed, stale
old-epoch reply ignored); `wasm-pack test` suites for library (11), pcb (31), case (52);
`python3 scripts/run-wasm-tests.py --files web/src/presentation/zmk_firmware_export.rs` — 5
passed; `cargo test -p boardstudio-application --locked` — 18 passed;
`cargo test -p boardstudio-web-runtime --locked` — 91 passed; `python3
scripts/check-wasm-tests.py`; `python3 scripts/check-doc-links.py`.

Follow-ups: `TestPersistence::saved_assets` is exposed but unused until ticket 05's mounted
tests need saved assets; the `OneShotBehavior` gate type can grow reply-kind filtering if a
later ticket needs it.
