# Proposed RF-014 — Host callback lifetime through browser terminal events

Status: confirmed production defect; isolated repair has five real Chromium tests passing. Independent Spec and Standards review cleared source commit `acdf17718069342b20d6d9438ad8df97b8bba57a`; packaged public acceptance remains pending. This finding is distinct from accepted-scope identity (RF-006).

Exact inspected baseline: `3c0cd7de2dadc6184c6b1771fbe0837a2aa2235a`, `web/src/host/storage.rs::transaction_completion`.

A real IndexedDB duplicate-key request dispatches `error` and later aborts the transaction. The production observer currently resolves its Rust future in the bubbling request-error handler, dropping the closure still installed as `IDBTransaction.onabort`. The later abort calls that dropped closure and raises `closure invoked recursively or after being dropped`. A focused Chromium test calls the actual private production observer against real IndexedDB; it reproduced the exact exception in 0.12 seconds. The result is a secondary host exception during persistence failure handling. An unpolled observer dropped by an early return is a second lifetime boundary being tested.

Historical public observation: source34723's second MatrixSetup save showed a transaction failure and five dropped-closure exceptions. The initial transaction abort cause is unknown. This confirmed callback defect does not prove quota exhaustion, explain that initial abort, or prove a source-level MatrixSetup defect. The original failed browser profile and pending state remain untouched.

Fresh public negative control: served source `3c0cd7de2dadc6184c6b1771fbe0837a2aa2235a` at34726 completed twelve 2×3 MatrixSetup saves, including Undo, Redo, reload, guide cancellation and compact-panel paths, ending at214parts with no JS errors. Evidence remains `/tmp/matrix-idb-diagnosis-20261002/`. This bounded passing stress run does not close the historical abort investigation.

Required correction: settle a transaction only at its terminal complete/abort event; retain callbacks through terminal dispatch and detach registered callbacks before their Rust owners drop, including unpolled/cancelled futures. Preserve caller-supplied abort attribution and native transaction failure details. Do not widen public APIs or bypass the existing persistence/outcome authority.

Required validation: actual Chromium production regressions for error→abort, dropped observer, handled request error that does not abort, ordinary completion and caller-attributed abort; affected strict checks; independent source review. A newly packaged public regression remains a separate acceptance gate. Carry this lifetime rule to other callback-backed host futures when their lifecycle is changed; do not infer all other observers are defective without evidence.

## Executed diagnosis and repair

Ranked hypotheses were (1) bubbling request error settles the future before transaction abort, (2) cancellation leaves installed closures after future drop, and (3) request callback re-entry. Three real production tests on the old implementation failed for the expected reasons: error→abort raised the exact dropped-closure exception; dropping an unpolled observer raised the same exception on completion; a handled request error returned `Err("IndexedDB transaction failed")` instead of the transaction's actual successful commit. Expanded red: 0 passed / 3 failed in 0.27s. This supports the first two hypotheses; no callback-reentry mechanism is needed to explain these reproductions.

The private observer now settles only on complete/abort. A private owning guard clears those event properties before dropping its Rust closures, including when the future was never polled. Abort uses the caller's recorded reason first, then the browser's DOMException name/message, then the generic explicit-abort fallback. No API/member visibility or suppression changes.

Green: all five real Chromium tests pass in 0.31s. Tests directly call the production observer with real IndexedDB, and independently read storage after completion/abort to verify durable write/rollback. They verify `ConstraintError` attribution and normal/aborted handlers being detached. The original transaction-abort trigger remains unknown; no pending browser state was discarded.

Reproduction command (from this isolated worktree):

```sh
TMPDIR=/home/chris/.local/share/boardstudio/tmp \
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/case-model-delivery-20261002/web/target \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner \
CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 \
WASM_BINDGEN_TEST_WEBDRIVER_JSON=/home/chris/.local/share/boardstudio/worktrees/matrix-setup-focus-repair-20261002/.scratch/dioxus-frontend-tranche-1/evidence/new17-compact-guide-repair-20261002/webdriver.json \
cargo test --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --lib host::storage::lifetime_tests
```

Use `--lib`: the storage observer lives in the WASM library. A bin-only filter executes zero tests and supplies no evidence.

Affected checks: actual Chromium WASM library tests 5/5 in the test profile; strict WASM page all-target Clippy (`-D warnings`) passed; formatter and production source diff checks passed. Commit-range whitespace checking reports runner-emitted trailing whitespace in the preserved raw evidence logs; those logs have not been normalized. Production edits are exclusively in the WASM host storage module; no native production path changed. The callback logic is not conditional on debug assertions.

Independent report: `/tmp/matrix-idb-observer-independent-review-20261002.md`. The storage host module is shared by WASM library/provider builds, so integration requires the complete package pipeline; page-only artifact reuse is insufficient. No provider or packaged public acceptance is claimed here.
