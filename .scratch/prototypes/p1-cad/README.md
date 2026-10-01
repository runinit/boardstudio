# P1-CAD isolated feasibility probe

**Current run: P1-CAD-F1-r1; bounded feasibility accepted at reviewed/integrated source8015b57f.**
The user's “do it” approves the bounded findings repair in
`evidence/findings-repair/PLAN.md`. Four isolated public-service controls prove
the old comparison mixed cold preview with export-warmed preview. The corrected
oracle retains exact vertex assertions, compares complete position/normal bytes
at matching independent worker lifetimes, and independently reopens STEP from
cold export and both cache orders. `docs/architecture.md` now records this
experimental boundary and its retirement criteria. Exact review/integration records are in `evidence/findings-repair/`; final administrative handoff review is separate. No production adoption follows.

**Historical original run: blocked, unaccepted.** The original reference gate
(`evidence/check-step.mjs`) fails on preview vertex count: isolated preview192
versus reference preview-after-export96. Bounds and volume agree. The existing
provider has different cold planar and export-warmed mesh paths; cache-state
mismatch was then only a source-supported explanation, not a validated passing gate.
Two diagnosed repair attempts are exhausted. Preserve the failed assertion and
worktree; no production or accepted migration integration follows. See
`evidence/blocker.json` and `evidence/repair-attempts.json`. Those records and the
original assertion in `evidence/findings-repair/original-check-step.mjs.gz`
remain unchanged. The renewed run has its own two-attempt limit and records.

This task implements only the approved P1-r1 CAD packaging investigation. It
copies the accepted P1-CORE package structure and a holed-plate test input. No
production module, manifest, provider visibility, kernel, contract or version is
changed. Nothing here is adopted automatically.

`src/worker.rs` owns separate generated CAD WASM instances: one worker for preview,
one for committed export. Rust imports the existing generated exports using
wasm-bindgen 0.2.129 `raw_module`. `build.py` produces initialization-only JS and
stages the separately built CAD JS/WASM beside the Rust worker. There is no
maintained JS CAD policy. `src/host.rs` owns worker handles, caller settlement and
completed mesh cache. The worker owns only executor-local completed cache metadata;
Dioxus owns the report signal. The document input is copied, prepared through the
public native `CoreEngine` example and immutable during each request.

The prototype preserves the existing `bodyKey` formula (including JS property
order/number spelling), existing generated CAD APIs and final-mesh tolerances.
An older valid completion populates the host cache before stale display rejection.
Each delta requires its exact completed cache base. Cancellation controls bypass
the serialized queue, and queued/active jobs settle without publishing or advancing
the completed index. Synchronous OCCT cannot be interrupted; cancellation is
observed at the next explicit yield between CAD calls. The kernel's private solid
cache may contain work computed before cancellation, matching the existing service.
Committed export uses a separate executor with a captured prepared input and no
preview-cache dependency. Close, crash and initialization failure settle pending
callers without replay. This is bounded fixture evidence, not production session,
undo/redo, cutover, scene or whole-workflow performance verification.

Transport is an opaque Rust serde_json text frame with float_roundtrip plus
separate owned ArrayBuffers. Prepared JSON is parsed into the existing CAD JS
object ABI; CAD serializes into its own WASM memory. CAD creates JS output arrays;
the Rust worker transfers their original buffers without an additional typed-array
copy. The host copies each received buffer once to Rust Vec storage shared with
the cache through Rc. STEP reopening copies Rust bytes to one JS Uint8Array, transfers
it (sender detaches), then CAD copies into its WASM memory. Multiple WASM runtimes
and these copies are deliberate prototype costs; no performance parity claim.

The existing CAD provider's JS Number revision ABI is limited above 2^53-1 (its
Rust return conversion casts u64 to f64). This probe uses revision 7 and preserves
that existing provider ABI. P1-CORE separately proves full-u64 JSON frames; this
probe does not establish full-u64 CAD compatibility. Resolving the CAD ABI and
complete M1 runtime/platform/performance gates remains outside this task.

Evidence records exact authority/base, preflight/routing smoke, RED/GREEN tests,
prerequisites/repairs, version-matched official API excerpts, builds, commands,
exit codes, actual browser URLs, artifact identities and independent reviews.
`evidence/site`, `target` and `worker-pkg` are generated outputs in this owned
worktree. Source copies and compressed evidence remain reviewable; failed output
is retained. Reproduce from this package after the approved provider prerequisites:

```sh
python3 evidence/run-checks.py release
node evidence/check-step.mjs
python3 evidence/run-checks.py final
```

The release harness uses real local Chromium through task-owned agent-browser
sessions, with a local Python server at root and `/boardstudio/`. It asserts real
mesh bounds/volume/normals, full/delta cache identities, active/queued cancellation,
retry and settlement, and buffer detachment/byte counts. STEP bytes are independently
reopened through the unchanged existing libcascade test oracle and compared with
an export from the existing TypeScript public service.
