# 009: Diagnostic gated-persistence adapter for packaged builds

Status: claimed
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Assignee: Pi Spark (009 implementation)
Blocked by: none — can start immediately (parallel with 008)

## What to build

The diagnostic-only gated-persistence adapter specified by
[Locate a deterministic packaged persistence hold](003-locate-packaged-persistence-hold.md):
a build-gated way for packaged verification to hold a packaged save genuinely
unanswered, then release it, fail it, or release it into failure, against the
real packaged store. It never ships as product behaviour. Exact diagnostic
A/B build flags, build-identity stamping and cleanup attestation are delivered
as part of this ticket.

## Acceptance criteria

- [ ] Arm hold: the save is entered with the intended name/revision, reports Saving, receives no acknowledgement and makes no durable change until released.
- [ ] Release completes the save and reports Saved at the intended revision; the reopened value is correct.
- [ ] Arm fail reports Failed, defers adoption as did-not-complete, and an explicit retry succeeds.
- [ ] Hold-then-fail is a single armed mode: releasing the hold enters failure atomically (not two re-armed gates).
- [ ] Persistence is judged by the real store's completion/abort acknowledgements, not request-success callbacks, save labels or worker labels.
- [ ] The adapter is diagnostic-only and build-gated; a normal packaged build contains no armed gate. Diagnostic build identity is distinguishable and attested, and disposable fixtures/databases are cleaned up with the cleanup attested in the completion record.
- [ ] Focused checks for touched crates pass; evidence logs are stored per the plan's evidence conventions.

## Implementation (009 delegate, 2026-11-07, HEAD `02ec0b6`)

Status stays claimed, not resolved: coordinator independent review still due.

### What landed

- New `web/crates/runtime/src/diagnostic_persistence.rs` (runtime-owned, 003 placement):
  `DiagnosticPersistGate` implements `DocumentPersistence` by delegating to the real
  `BrowserStore::save_document_expected`; target fixed to Store. Single-armed modes
  `hold` / `fail(reason)` / `hold_then_fail(reason)` taken at persist entry
  (pre-transaction, after Core work, before any durable write — the demonstrated hold
  point). Hold parks on a oneshot waiter; `release()` completes against the real store
  or fails atomically for hold-then-fail (no re-arm); `disarm()` clears the pending arm;
  `snapshot()` polls `{armed, entered, document_id, document_name, revision,
  expected_token_present, released, acked}`. Ack judgement is the store's own
  `Ok`/`Err` (transaction complete/abort), recorded per save. Window surface
  `window.__boardstudioPersistGate` (`arm_hold`, `arm_fail`, `arm_hold_then_fail`,
  `snapshot`, `release`, `disarm`, exact `js_name`s) registered from `wrap_store()`,
  driving the latest constructed gate; worker, Session and save policy untouched.
- Minimal shared touches: `runtime.rs` persistence composition is a cfg branch
  (feature off keeps the exact existing expression); `lib.rs` one cfg module decl;
  `web/crates/runtime/Cargo.toml` empty `diagnostic-persist-gate` feature;
  `web/Cargo.toml` forwarding feature of the same name; `scripts/build-web.py`
  `--diagnostic-persist-gate` flag plus a matching test in `scripts/test-build-web.py`.
  No member or public-API widening. No 008/010–015/019 files touched.

### RED → GREEN (no phase approvals used; no manufactured pass)

- RED (`.evidence/update-draft-safety/009-red-wasm.log`): 6 tests, 2 pass
  (passthrough-transparency guard, build flag), 4 fail for the expected missing
  behavior (hold completed despite arm; fail returned `Ok(token)`; hold-then-fail
  completed; window arm unrecorded).
- GREEN (`.evidence/update-draft-safety/009-green-wasm.log`): 6/6 pass, rc=0, zero
  warnings. Tests found two real RefCell-guard bugs (match/`if let` scrutinee guards
  living across arms); both fixed by hoisting takes into `let`s.
- Regression: full runtime wasm suite 65/65 with the feature armed-off
  (`009-regression-feature-wasm.log`) and 59/59 without it
  (`009-regression-plain-wasm.log`); native runtime 138/138 (`009-native-runtime.log`).
- Focused checks: rustfmt clean (`009-fmt.log`); Clippy `-D warnings` wasm32 all-targets
  clean both states (`009-clippy-plain.log`, `009-clippy-gate.log`); page typecheck
  clean both states (`009-pagecheck-plain.log`, `009-pagecheck-gate.log`); tooling
  tests 9/9 (`009-tooling-tests.log`).

### Exact diagnostic build flags, identity, cleanup

- Normal: `dx build --web --base-path <prefix> --no-default-features --features page
  --cargo-args=--locked` (i.e. `python3 scripts/build-web.py [--fast] [--routes …]`).
- Diagnostic: same with `--features "page diagnostic-persist-gate"` (i.e. add
  `scripts/build-web.py --diagnostic-persist-gate`). `dx --features` takes a
  space-separated list (verified in `dx build --help`); the comma form was not used.
- Packaged proof (fast checked builds, both routes): normal wasm contains 0 occurrences
  of `__boardstudioPersistGate`, diagnostic wasm 1, on both routes; served normal page
  reports `typeof window.__boardstudioPersistGate === "undefined"`; served stamped
  diagnostic A reports an object whose `snapshot()` round-trips
  unarmed → `arm_hold()` → `armed:"hold"` → `disarm()` → unarmed.
- Identity: diagnostic output stamped to A/B with `verify-update-adoption.py`
  `stamp_build` (meta + `boardstudio-build<A|B>` wasm custom section + worker re-embed);
  packaged diff is exactly `index.html` + `service-worker.js` + `wasm/boardstudio-web_bg.wasm`
  on both routes with distinct versioned manifests (`009-stamp.log` with sha256s).
- Cleanup attested: agent-browser session closed; servers stopped with ports 4771/4772
  verified free; `web/target/site-009-normal`, `site-009-diag`, `site-009-ab` and the
  `/tmp` payloads removed (all git-ignored/disposable); per-test IndexedDBs deleted
  inside the tests; no project was ever opened in packaged probing. No source
  trees/targets/manifests were placed under `.evidence/` (logs only).

### Acceptance mapping and honest gaps

- Hold / release+Saved-value / fail+retry / single-armed hold-then-fail+reusable are
  proven at the persistence port against the real BrowserStore with completion/abort
  acks and reopened-value checks (6 wasm tests). Runtime `Saving`→`Saved{revision}`
  mapping rides the untouched shared `Runtime::run` Persist arm; its hold shape is
  covered by the existing `gated_save_holds_saving_until_released_then_reports_saved`
  test, and full toast/header legs with trusted input belong to 010's packaged proof.
- Gortex: pre-edit impact (MEDIUM, 43 transitive lower bound, no signature change);
  post-edit detect mapped 4 files; tests op shows coverage; guards unconfigured;
  contract verdict `warn` (behavioral port change, blast reviewed, no parse errors).
- Blocker encountered and cleared: Gortex mutation admission rejected 3× as stale
  after the HEAD change; restored after ~4 min of independent env/tooling checks with
  bounded retries, no bypass. No new visibility/API was needed beyond the 003-approved
  additive adapter contract.
