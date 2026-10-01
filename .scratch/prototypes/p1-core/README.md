# P1-CORE isolated feasibility probe

Authority: [P1-r1](../../../tasks/plan.md), approved six-capability specs at
`47d6dbce5af2285d1f886ef63d2d4c7b1de57925`, source engine base `96dd51d3`,
advance user approval recorded in [RUN](../../../docs/migration/RUN.md).
The original failed run is retained at `/tmp/boardstudio-p1-core-20261001`,
branch `prototype/boardstudio-p1-core`, candidate `3f25e9d9`, evidence `6519a8c2`.
The renewed P1-U64-r1 repair is authorized by the explicit 2026-10-01 user request
`$ask-matt resolve this u64 serialization falure`, with
[bounded authority](evidence/u64-repair/task-start.json). Its task-owned worktree is
`/home/chris/.local/share/boardstudio/worktrees/p1-u64-repair-20261001`, branch
`prototype/boardstudio-p1-u64`, integration base `8b293b08`.
Only this subtree and coordinator-owned task records are changed.

This is transport/package feasibility, preserving existing provider semantics.
The worker owns one real CoreEngine and calls its typed `handle` method. The
host owns pending transport identities and read-only replies; its Dioxus Signal
holds only a test report. `host.rs` owns the browser registrations and handles;
Drop clears registrations and terminates its worker. The worker callback is
retained for that worker's lifetime and released when the worker is terminated.
No session/store/persistence/CAD/renderer implementation is promoted or adopted.

`lib.rs` defines only this throwaway transport/test envelope and caller ledger.
Native tests first failed on missing implementations; all four then passed.
Core Open/Snapshot meanings are reused, with their original request/reply IDs.
External executor epoch/operation IDs reject stale/unsolicited replies. Fail/close
settles pending operations and closes the executor without replay. Crash and
missing-worker cases deliberately generate worker errors and confirm caller
settlement; page-error inspection must still be empty. No historical revision,
transaction ID or mutation deduplication contract is changed by this probe.

The copied input is an empty `boardstudio/v2` document created by the existing
public constructor, named `probe-copy`. It is sufficient to observe a real
stateful Open/Snapshot round trip and insufficient to prove M1 geometry, Undo,
archives, persistence, CAD parity or performance. Those gates remain open.

## Reproduce

From this directory, using the pinned toolchain and installed CLI:

```sh
cargo test --locked
cargo build --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --lib --target wasm32-unknown-unknown --no-default-features --features worker -- -D warnings
cargo clippy --locked --bin boardstudio-p1-core --target wasm32-unknown-unknown -- -D warnings
wasm-pack build . --target web --release --out-dir worker-pkg --locked --no-default-features --features worker
dx build --web --release --base-path / --cargo-args=--locked
```

Read the actual CLI-reported public artifact directory, then:

```sh
python3 build.py /absolute/cli/reported/public /
python3 evidence/verify-browser.py /
dx build --web --release --base-path /boardstudio/ --cargo-args=--locked
python3 build.py /absolute/cli/reported/public /boardstudio/
python3 evidence/verify-browser.py /boardstudio/
```

Every wasm-pack-owned option must precede its Cargo EXTRA_OPTIONS. The original
illustrative plan placed `--out-dir` too late; the first build failed accordingly.
The corrected command and failure are retained. A second diagnosed repair
collapsed a nested host conditional to satisfy WASM Clippy; no lint exemption
or assertion change was used. That original task exhausted its two repairs and
remains a recorded failure. The renewed defect-only task has a separate limit
of two diagnosed repairs; it does not authorize CAD work or production adoption.

`build.py` emits initialization-only generated JS (`init`, then `start_worker`)
and stages the CLI host and fresh wasm-pack worker artifacts under the two
explicit prefixes. Engine/transport policy stays in Rust. No worker support is
attributed automatically to Dioxus. Browser sessions, ports and static server
processes belong to the harness; it closes its own resources, with no cloud
browser, existing data origin, push or deployment.

## Evidence and remaining costs

The original `evidence/*.json` and old build logs are immutable historical
provenance, including the original failed precision diagnostics. Their frozen
`check-large-integer-contract.py` checks the old failure record;
`verify-large-integer.py` deliberately expects the old bug. Neither is a fresh
passing gate. The current executable gate is `verify-browser.py`, which retains
all original lifecycle/transfer assertions and adds typed precision assertions
through Client.send and the real CoreEngine worker. Current RED, corrected
builds, browser verdicts, hashes and checks are under [u64-repair](evidence/u64-repair/).


See [task authority and attempts](evidence/task-start.json),
[native/WASM checks](evidence/final-checks.json),
[release builds and browser checks](evidence/repaired-build-browser.json),
[provider regression reuse](evidence/provider-evidence.json),
[artifact hashes](evidence/artifacts.json) and [versioned APIs](evidence/api-evidence.json).
Root and subpath JSON records include runtime Open/Snapshot, three rejected
invalid/stale/unsolicited replies, close/crash/init settlement and sender buffer
byte length zero with received `[3, 1, 4]`. They retain actual browser commands
and page-error inspection. Generated binaries/site are local artifacts, ignored
by Git; hashes, source and executable producer remain reviewable.

The original structured-object codec rejected valid u64/i64 values outside the
JavaScript safe-integer range. Frames now carry opaque JSON text encoded and
decoded with the already-locked Rust serde_json 1.0.151. JavaScript never parses
that text or converts its numbers. The real Rust host/worker regression checks
Open and Snapshot at revisions 9007199254740991, 9007199254740993 and u64::MAX,
including scene revisions, nested u64/i64 extrema, null, booleans, strings,
fractional floats, 1.0 and negative zero. Typed and serialized-value equality
are asserted in Rust; only descriptive strings reach the browser report.

BigInt serializer configuration is an alternative for large integer fields,
but pinned serde-wasm-bindgen deserialize_any normalizes safe integral JS numbers
to i64 and loses whole-float/negative-zero JSON representation. A tagged numeric
schema would add a new protocol and validation surface. JSON text reuses the
provider codec without new dependencies or changes to engine types. See
[version-matched installed sources](evidence/u64-repair/codec-source-evidence.json).
The direct serde-wasm-bindgen pin and lockfile remain unchanged in this disposable
probe; transport no longer calls it. This avoids mixing dependency changes into
the defect repair.

Intentional wire change: frame is a JSON string rather than a JS structured
object. Both endpoints must be built together; this probe has no persisted or
public wire consumers. Malformed/non-string replies are still rejected, and
worker decoding failures still settle callers through the existing error path.
Operation identity, stale rejection, provider Open/Snapshot semantics, lifecycle
and transfer ownership remain unchanged. Document/request/reply values still
serialize and copy across JS/WASM and worker boundaries, with JSON parsing cost;
this fix makes no performance claim. The three-byte
buffer is copied from Rust into a Uint8Array, transferred without an intervening
structured-clone byte copy, transferred back, then copied to a Rust Vec for the
assertion. This proves transfer ownership only. It gives no hot-path copy,
latency, retained-memory or M1 performance budget. Core and host are separate
WASM modules and can duplicate dependency/module memory; measure representative
workloads before adoption. Cancellation of synchronous engine work is outside
this transport proof; no interruptibility guarantee is inferred.

Integration requires fresh exact-candidate review and resulting-revision checks.
A successful verdict only makes P1-CAD eligible; it does not adopt this code.
