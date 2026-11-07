# 009: Diagnostic gated-persistence adapter for packaged builds

Status: resolved
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

- [x] Arm hold: the save is entered with the intended name/revision, reports Saving, receives no acknowledgement and makes no durable change until released.
- [x] Release completes the save and reports Saved at the intended revision; the reopened value is correct.
- [x] Arm fail reports Failed, defers adoption as did-not-complete, and an explicit retry succeeds.
- [x] Hold-then-fail is a single armed mode: releasing the hold enters failure atomically (not two re-armed gates).
- [x] Persistence is judged by the real store's completion/abort acknowledgements, not request-success callbacks, save labels or worker labels.
- [x] The adapter is diagnostic-only and build-gated; a normal packaged build contains no armed gate. Diagnostic build identity is distinguishable and attested, and disposable fixtures/databases are cleaned up with the cleanup attested in the completion record.
- [x] Focused checks for touched crates pass; evidence logs are stored per the plan's evidence conventions.

## Implementation (009 delegate, 2026-11-07, HEAD `02ec0b6`)

Status stays claimed, not resolved: initial Spec and Standards reviews requested
changes; the Sol follow-up below awaits fresh coordinator review.

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
  `/tmp` payloads removed (all git-ignored/disposable); per-test IndexedDB deletion
  was only scheduled in this initial run, not completion-attested (corrected by the
  follow-up below); no project was ever opened in packaged probing. No source
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

## Required review follow-up (Pi Sol, 2026-11-07, base `043787ba5`)

Both required findings are implemented, **not independently accepted yet**. Status
and acceptance checkboxes remain unchanged pending the coordinator's fail-path rerun
and fresh Spec/Standards recheck. Only this ticket and the adapter's `cfg(test)`
section changed; all product code, build flags, module metadata, worker/Session
policy, 011 work and the coordinator's untracked `execution.md` are untouched.

### Finding dispositions

1. **Deterministic entered observation:** both hold tests now poll `snapshot().entered`
   with a 5-second deadline and 5-ms yielding cadence, matching the existing canonical
   browser-test wait pattern without adding a dependency on downstream UI support or
   a general wait framework. Each poll also drives the owned save future and rejects
   an acknowledgement before release. Only after observed entry do the tests check
   no acknowledgement and the unchanged real store. The ordinary hold deliberately
   enters after 250 ms, beyond the old 150-ms guess; that timer supplies latency, not
   readiness evidence. Release acknowledgement is also deadline-bounded. Save futures
   are local and dropped on a returned check failure, not detached tasks able to
   recreate a database after fixture cleanup. This is a test-reliability correction,
   **not a new product-behavior RED claim**; the original 4-fail/6 → 6-pass evidence
   above is preserved.
2. **Real cleanup attestation:** `delete_database` now awaits the actual IndexedDB
   deletion success event, returns errors on `error` or `blocked`, and fails after a
   5-second deadline if no outcome arrives. A local RAII guard detaches all three
   callbacks on return/cancellation; the first observed outcome is retained, so later
   success cannot erase a reported error/blocked condition. The five original
   database-owning tests return check/operation failures through `in_database`, which
   awaits deletion before reporting either the test failure or a cleanup failure
   (including both when both fail). Window-test registration is also removed.

### Cleanup proof and runtime boundary

- The real open-connection control rejects blocked deletion, then closes the
  connection and awaits successful deletion. A second real deletion queued behind a
  blocked request exercises timeout and callback detachment; after closing the
  blocker, final deletion is awaited before assertions.
- A deliberately failed check after a real save returns its original failure **after
  cleanup**. Reopening raises `upgradeneeded`, independently proving the fixture
  database was absent rather than merely scheduled for deletion. That verification
  connection is closed and its recreated database is also deletion-acknowledged.
- The error-handler control dispatches a **synthetic error event** on a real request;
  it proves error reporting, first-outcome retention and detachment, not a real browser
  backend deletion fault. A subsequent real deletion is awaited before assertions.
- Passing runs attest deletion completion for every disposable fixture. Returned
  assertion-equivalent checks and storage errors take the same awaited cleanup path.
  Unexpected Rust panic-abort, JavaScript traps, a killed runner, or a never-returning
  test body cannot execute asynchronous cleanup; no async-`Drop` or all-panics cleanup
  guarantee is claimed. The controls' final harness assertions occur after cleanup.
  No live application profile/project was used, and no packaged server/output was
  created in this follow-up. Earlier packaged-output cleanup is historical evidence,
  not a newly rerun packaged proof.

### Focused evidence

All paths below are under `.evidence/update-draft-safety/` (logs only).

| Run | Result | Log |
| --- | --- | --- |
| Unchanged baseline | 6 passed, 0 failed | `009-sol-baseline.log` |
| Cleanup RED, scheduling-only helper | 6 passed, 1 expected failure: blocked deletion falsely returned success | `009-sol-cleanup-red.log` |
| Cleanup GREEN | 7 passed, 0 failed | `009-sol-cleanup-green.log` |
| Error-event control during helper development | 9 passed, 1 failure: later success overwrote the earlier error; fixed by retaining first outcome | `009-sol-error-event-red.log` |
| Final adapter suite (original six + four cleanup controls) | 10 passed, 0 failed, 59 filtered | `009-sol-final-wasm.log` |
| Existing Runtime held-save Saving → Saved regression | 1 passed, 0 failed, 68 filtered | `009-sol-runtime-regression.log` |
| Diagnostic Clippy, wasm32 all targets, `-D warnings` | passed | `009-sol-clippy-gate.log` |
| Normal Clippy, wasm32 all targets, `-D warnings` | passed | `009-sol-clippy-plain.log` |
| Adapter rustfmt check | passed | `009-sol-fmt.log` |
| Documentation links | 717 Markdown files passed | `009-sol-doc-links.log` |
| Diff whitespace check | passed | `009-sol-diff-check.log` |

Exact final commands (each stdout/stderr redirected to its matching log above):

```sh
CARGO_PROFILE_DEV_DEBUG=false WASM_BINDGEN_TEST_TIMEOUT=120 wasm-pack test --headless --chrome web/crates/runtime --lib --features diagnostic-persist-gate -- diagnostic_persistence::tests::
CARGO_PROFILE_DEV_DEBUG=false WASM_BINDGEN_TEST_TIMEOUT=120 wasm-pack test --headless --chrome web/crates/runtime --lib --features diagnostic-persist-gate -- gated_save_holds_saving_until_released_then_reports_saved
cargo clippy --locked -p boardstudio-web-runtime --target wasm32-unknown-unknown --all-targets --features diagnostic-persist-gate -- -D warnings
cargo clippy --locked -p boardstudio-web-runtime --target wasm32-unknown-unknown --all-targets -- -D warnings
rustfmt --edition 2024 --color never --check web/crates/runtime/src/diagnostic_persistence.rs
python3 scripts/check-doc-links.py
git diff --check
```

Baseline and RED/intermediate adapter runs used the same first command. An initial
compile-only failure (ambiguous `Reflect::delete_property` object type) is retained in
`009-sol-compile-initial.log`; it is not behavioral RED evidence. No full gate, full
build or fresh packaged A/B run was performed; coordinator 015 owns the full gate.
The unchanged build-tooling 9/9 coordinator result at `86bd4620b` is reused, not
claimed as a new run.

Native Gortex was used for source inspection and all edits: pre-edit impact,
`delete_database` signature verification (5 callers checked, clean), post-edit detect,
tests/guards/contract and caller relations. Automatic route reconciliation temporarily
blocked exact mutations and recovered without tracking, daemon changes or fallback.
Detect maps the changes to the test module; guards have no configured rules. Contract
returns `warn`/high lower-bound blast; caller inspection confines the helpers to this
private test module. Its tests operation reports 0/5 helper coverage because these
`wasm_bindgen_test` functions are indexed as `is_test:false`; the executed named
10-test browser result above supplies coverage evidence, not a fabricated graph pass.

Optional findings were deliberately left alone: fail-arm entered identity, mid-hold
rearm/disarm semantics, compile-presence versus packaged identity, and arm/reset
repetition. No new production mode, member visibility or window API was added.

## Answer

Coordinator acceptance (2026-11-07): `86bd4620b` and required correction
`764a4dd54` are accepted after independent Spec/Standards review and coordinator
reruns: adapter **10/10**, Runtime held-save **1/1**, tooling **9/9**. See
[execution](../execution.md#verified-evidence-to-date) and
`009-coordinator-sol-{gate,runtime}.log`, `009-coordinator-tooling.log`.
The normal-package gate-absence proof above is reused, not claimed as a new run.
[010](010-packaged-a-to-b-proof.md#answer) supplies actual packaged hold/release,
both literal Retry paths, gate reuse in B, exact durable revisions and final
profile/database/output cleanup. All seven criteria are met; historical
review-pending statements above are superseded. Diagnostic fast runs do not
establish optimized equivalence; that and final integrated verification remain
015 obligations. No shipping feature flag or persistence policy changed.
