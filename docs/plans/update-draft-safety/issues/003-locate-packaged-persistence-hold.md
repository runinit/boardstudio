# 003: Locate a deterministic packaged persistence hold

Status: resolved
Type: task
Label: planning:task
Mode: AFK
Assignee: Chris D
Blocked by: none
Parent: [Draft-safe update adoption](../map.md)
Related: [Draft-safety research and change map](../../../research/2026-11-07-019-draft-safety-change-map.md)

## Question

How can a packaged diagnostic fixture deterministically hold, observe and release a
real project save before Runtime receives persistence acknowledgment, without replacing
browser storage or the worker lifecycle? The answer unblocks a meaningful verification
protocol; shipping a new persistence or adoption mechanism is not the task.

## Evidence to obtain

- Trace the existing `DocumentPersistence`/browser-store seam and `gate_next_persist`
  support from current dirty source. Distinguish Core work, persist entry, transaction
  state, acknowledgment, accepted revision and retained save; do not hold only Core.
- Establish whether existing support can drive a packaged fixture. If a narrow
  test-only adapter is needed, identify its placement, build/instrumentation difference,
  ownership and cleanup. State whether the hold is before a transaction, in flight,
  or after commit with acknowledgment withheld; do not conflate those states.
- Demonstrate the probe can signal entered, remain unanswered, release and observe
  the intended actual saved value. Identify a failure/abort control and the evidence
  needed to check deferral, cancellation, takeover-time work and reopening in B later.
- No sleep, fabricated save label or fake storage completion substitutes for the
  acknowledgment. Keep existing real browser storage, adoption and cache behavior.

## Completion and boundaries

Return a concrete supported probe/adapter and exact reuse protocol, or an exact blocker
and smallest materially different next experiment. Leave the ticket unresolved when
blocked; a blocker transfer is not completed evidence. Use disposable fixtures and scopes,
coordinate browser ownership, and serialize builds per target. Test-only work must not
ship hooks or widen production/member visibility without approval. Do not run a full
acceptance gate, implement the safety fix or claim packaged held-save safety from the
probe alone. Claim first; answer here with source IDs and supporting asset/log links.

## Answer

Resolved 2026-11-07. **The deterministic hold already exists at the persistence
port and is demonstrated in a real browser; it cannot drive the packaged app
today, and a narrow diagnostic-only adapter at one named composition point is
the concrete route for packaged use.** No product code changed; two Pi
`meta/muse-spark-1.3-contributor` evidence delegates plus orchestrator Gortex
verification and fresh serialized browser-test runs (logs:
`.evidence/update-draft-safety/003-wasm-gated-save.log`,
`.evidence/update-draft-safety/003-wasm-notifications.log`,
`.evidence/update-draft-safety/003-wasm-store-target.log`).

### Where the hold sits (do not conflate the states)

Trace, all current dirty source: `Runtime::run` Persist arm
(`web/crates/runtime/src/runtime.rs:2232-2263`, ID
`…/runtime.rs::Runtime.run`) sets retained `pending_document`, clones
`Rc<dyn DocumentPersistence>`, awaits `save_document`; only on `Ok(token)`
does it update `saved_tokens`, clear the retained save and deliver
`Completion::Persist` → `Session::persist_completed`
(`application/src/session.rs:1966`). The production adapter is
`DocumentPersistence for BrowserStore` (`runtime.rs:242-263`):
`BrowserStore::save_document_expected`
(`web/crates/host/src/host/storage.rs:158`) → `save_assets_expected`
(`storage.rs:178`, token minted from the serialized document **before**
commit, returned only after) → `persist_project` (`storage.rs:208`, free fn
`storage.rs:558`) → `transaction_completion` (`storage.rs:959`): the future
resolves on IndexedDB transaction `oncomplete`/`onabort`, **not** on a
put-request `onsuccess`. So the real acknowledgment is post-commit, and the
only app-controllable hold points around it are before the port call or after
`persist_project` returns.

The existing gate is **pre-transaction, at persist entry — after Core work,
after `Effect::Persist`, before any transaction, token delivery or accepted
revision**: `TestPersistence::save_document`
(`web/crates/runtime/src/in_process_support.rs:382-432`, ID
`…/in_process_support.rs::TestPersistence.save_document`) takes the one-shot
`Hold` behavior, sends `entered`, awaits `release`, and only then delegates
to its target; `gate_next_save` (`in_process_support.rs:344`) is exposed as
`project_name_test_support::gate_next_persist` (`runtime.rs:8664`). The
target is the **real** `BrowserStore` (`PersistTarget::Store`,
`in_process_support.rs:20,321`) unless `install_memory_persistence`
(`runtime.rs:8491`) switches it to memory. While held, Runtime stays
`Durability::Saving` with the save retained — it is not a Core-only hold, not
an in-flight transaction, and not a committed-but-unacknowledged write.
Fresh demonstration (headless Chrome, serialized, this session):
`gated_save_holds_saving_until_released_then_reports_saved`
(`runtime.rs:10129`) 1 passed — entered awaited, `Saving` asserted, release →
`Completed`, `Saved{revision}`, intended `Gated save` value in the store;
`pending_store_save_keeps_its_target_while_future_saves_switch_to_memory`
(`web/crates/runtime/src/cad_runtime_borrow_tests.rs:52`) 1 passed — a save
captured on the real store target lands in IndexedDB (`load_document` reads
it back), so the hold-then-delegate target is genuinely BrowserStore;
`presentation::notifications::tests::` 17 passed, including
`mounted_update_now_defers_while_a_save_is_in_flight`
(`web/src/presentation/notifications.rs:1127`: gated save → Update Now shows
the save-in-progress deferral, adoption count 0, Cancel, release →
`Saved{revision: 1}`) and its failed-save sibling (`:1165`). Those mounted
tests use the memory target (via `open_document`, `runtime.rs:8497`), so
they prove the gate/ack semantics, not packaged behavior.

### Why existing support cannot drive a packaged fixture

- **Compiled out:** `project_name_test_support` (`runtime.rs:8449-8450`),
  `in_process_support` (`runtime.rs:7861-7863`) and `GateDriver`
  (`web/crates/runtime/src/lib.rs:19`) are `cfg(any(test,
  feature = "test-support"))`; `test-support = []`
  (`web/crates/runtime/Cargo.toml:11`) is enabled only by dev-dependencies
  (`web/Cargo.toml:49-57`). The packaged build is `dx build --features page`
  (`scripts/build-web.py:171`) — no `test-support`.
- **In-process only:** `entered`/`release` are Rust oneshot channels behind
  `installed(runtime)` (`runtime.rs:8640`), which requires
  `InProcessAdapters` — installing them also replaces the Core **worker**
  with an in-process `CoreEngine`, which this ticket forbids, and nothing in
  the production page calls `install` or `gate_next_persist`; there is no
  external arm/entered/release surface for a packaged driver.
- **Composition is hardwired:** production `Runtime::new_with_store`
  (`runtime.rs:452-464`) sets `persistence: Rc::new(store.clone())`
  directly; there is no production-accessible persistence hook, gate or flag
  (searched: no seam on any production signature in `storage.rs`).

### The narrow test-only adapter (for the later verification ticket)

- **Placement/ownership:** runtime crate owns it. New
  `#[cfg(feature = "diagnostic-persist-gate")]` module (suggested
  `web/crates/runtime/src/diagnostic_persistence.rs`, feature declared empty
  like `test-support` in `web/crates/runtime/Cargo.toml`) implementing
  `DocumentPersistence` by delegating to `BrowserStore::save_document_expected`
  — browser storage, worker lifecycle, Session and save policy untouched.
  The single production touch is the cfg branch at the `persistence:`
  composition in `new_with_store`; shipping builds (feature off, and
  `scripts/build-web.py` unchanged) compile it out entirely. No member or
  public-API widening: the control surface is a `window` object registered
  only in the diagnostic build.
- **Semantics:** generalize the `TestPersistence` one-shot behavior with the
  target fixed to `Store`: modes `hold`, `fail(reason)`, `hold_then_fail`
  (existing `gate_next_persist`+`fail_next_persist` cannot queue hold-then-
  fail — `GateDriver` is single-shot, `in_process_support.rs:340-350`; the
  Core side already shows the combined pattern at `runtime.rs:8689`). Hold
  stays **pre-transaction** as demonstrated. Do **not** build the first
  probe as after-commit ack-withholding: at that point IndexedDB has
  committed, `active_key` is set and `observed` already holds the new token
  while Runtime still shows `Saving`, and a retry with the old expected
  token conflicts (`storage.rs:178-206` + free `persist_project` version
  check); an in-transaction pause is not available from app code at all.
- **Control surface (diagnostic build only):**
  `window.__boardstudioPersistGate` with `arm_hold()`, `arm_fail(reason)`,
  `arm_hold_then_fail(reason)`, `snapshot()` → `{armed, entered, document_id,
  document_name, revision, expected_token_present, released, acked}`,
  `release()`, `disarm()`. `snapshot` is polled for state (no sleep-based
  assertions); entered captures the intended saved value's identity at
  persist entry.
- **Exact reuse protocol for later packaged verification (B):** build
  disposable diagnostic A and B with the existing
  `scripts/verify-update-adoption.py` stamping plus the diagnostic feature
  (002's isolated profile/demo-copy discipline; serialized builds; one
  browser owner). In A: open the disposable project, Apply a baseline edit,
  observe `Saved` at revision r₀. `arm_hold()`; make the real admitted edit;
  await `snapshot().entered` with the intended name/revision; assert
  `Durability::Saving` (save label + no ack) and no durable change yet;
  click toast Update Now → expect the save-in-progress deferral, Cancel
  (adoption/reload counts stay 0); `release()` → await ack, `Saved` at r₁,
  and the reopened document holding the intended value. Failure leg:
  `arm_fail("diagnostic quota")` → `Failed`, Update Now defers as
  did-not-complete, explicit retry succeeds; `hold_then_fail` covers
  release-into-failure. Post-takeover/reopen checks consume the same gate
  in build B. Cleanup: feature-gated code never ships; delete the
  disposable profile/database and diagnostic output after the run.

This locates the hold and names the instrumentation limit; it is **not** a
packaged held-save safety claim — that proof waits on building this adapter
under the verification plan in [Agree the verification plan and
implementation transfer](006-agree-verification-and-implementation-transfer.md).
No new ticket graduates: the adapter and protocol are already specifiable
inside that ticket, and [Choose the draft-safety interface and lifetime
contract](004-choose-draft-safety-interface.md) is now unblocked. No
product implementation, no full acceptance gate.
