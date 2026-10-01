# BoardStudio continuation: P1 worker/CAD packaging

Revision: **P1-r1**, 2026-10-01. This extends [PLAN.md](../PLAN.md) and the
existing approved P1 charter; it is the next bounded integration investigation,
not a competing or whole-migration plan. Canonical task status: [root TODO](../TODO.md).

## Exact authority and scope

Specifications: [host-platform](../SPEC-host-platform.md),
[document-engine](../SPEC-document-engine.md),
[generators-cad-export](../SPEC-generators-cad-export.md),
[editor-session](../SPEC-editor-session.md), with rendering/presentation only
where their transport/packaging constraints apply. Architecture: accepted
[ADR 0003](../docs/adr/0003-rust-application-ownership.md). Core source base is
`96dd51d3e790c28f5554a8c9888a147c8814e2a7`.

Recorded user approval: **“Assume everything is approved in advance”**;
CLI installation additionally has **“Install it ...?”**. The coordinator records
the exact document candidate commit/hash in [RUN.md](../docs/migration/RUN.md)
before executing P1. Approval permits local task commits and integration into
the dedicated migration branch; it never permits main integration, push or
deployment. Existing protected worktrees remain read-only.

P1 asks whether the pinned CLI can package a Rust host and real core/CAD workers
using existing public providers at root and `/boardstudio/`. Only throwaway
probe packages, copied inputs and evidence are written. No application crate,
session implementation, production manifest/API change, new renderer backend,
database migration or automatic prototype adoption is included. P2/P3 runtime
work is separate; complete M1 remains blocked until its required gates pass.

## Dependency graph and ownership

TOOL-1 (completed) + DOC-1 technical validation → **P1-CORE** → **P1-CAD**.
These tasks share manifest/transport/assets and must be serialized. No parallel
implementation is justified. At most two implementation workers may be active,
with a separate reviewer slot; only the coordinator delegates and updates
integration/canonical status. If thread closure is unavailable, do not exceed
the three-thread cap or silently change models; coordinator work remains allowed.

The coordinator creates each task's worktree/branch from the recorded integration
commit, verifies actual cwd/HEAD/status, and assigns one task only. Each has
its own `.scratch/prototypes/` subtree. For workers, use Luna/high; narrow
read-only inventory uses Luna/medium. Review uses Luna/high with no authorship,
no writes and no child agents. Existing Codex session remains coordinator.

## P1-CORE: real core worker and pinned web packaging

Category: isolated feasibility prototype preserving public provider behavior.
Own `.scratch/prototypes/p1-core/`; likely authored files are Cargo manifest/
lockfile, Rust host, Rust worker library and build/bootstrap producer. Generated
assets and task evidence stay under this subtree/evidence directory. No source
outside it may change except coordinator-owned status/evidence documents.

Acceptance:

1. Pinned Dioxus 0.7.10 minimal/web/mounted and binding pins build a release
   host plus a real core worker. Typed `CoreEngine::handle` runs in the worker;
   existing public request/reply encodings retain their meaning.
2. At root and subpath, actual browser initialization and Open/Snapshot replies
   succeed. External operation/executor identity and invalid/stale replies are
   observed; close/crash/init failure settle callers without replaying mutations.
3. Transfer an owned test buffer and demonstrate sender detachment/receiver bytes;
   record remaining serialization/copy costs. No geometry or M1 parity claim
   follows this transport proof.

Verification: native meaningful protocol tests, Rust formatting/Clippy, release
worker/host builds, real Chromium root/subpath request/lifecycle checks and a
fresh exact-candidate quality review. Define/check the failing protocol or
browser expectation before implementing its replacement. Pure copies of existing
engine behavior reuse native provider evidence; new transport/lifecycle behavior
requires its own assertions. No silent test/check omission.

## P1-CAD: existing CAD package in Rust-managed worker

Category: isolated feasibility prototype preserving exact provider semantics.
Depends on accepted P1-CORE integration. Own `.scratch/prototypes/p1-cad/`;
reuse the proven package structure as a copied experimental input, not production
code. Author only its manifest/lockfile, host/worker and bounded harness/input
record, approximately five source files; generated CAD assets/logs are separate.

Acceptance:

1. Existing pinned CAD package initializes through its generated WASM exports
   under a Rust-managed worker at both prefixes, without widening private APIs
   or introducing authored JS CAD policy. Record actual URLs and source/artifact
   identities; build fresh eligible artifacts when provenance is insufficient.
2. One copied prepared-case input generates meshes and committed STEP through
   existing services. Compare existing reference geometry/export oracles and
   cache/full/delta identities; do not claim whole-workflow performance parity.
3. Supersession/cooperative cancellation, raced cache updates and worker
   close/crash settle callers correctly. Owned-buffer transfer/copy measurements
   and missing runtime evidence are explicit.

Verification: prerequisite/native CAD tests, fresh CAD/host/worker release build,
existing relevant CAD cache/STEP regressions, real browser root/subpath lifecycle
and output assertions, and independent exact-candidate review. If separate
CAD-package interop fails, capture the cause and block P1-CAD; a direct-link/kernel/
visibility alternative is not silently substituted.

## Executable commands and evidence

From repository root, provider commands remain those in the specs/constraints:

```sh
cargo test --manifest-path core/Cargo.toml --locked
cargo build --manifest-path core/Cargo.toml --locked --example prepare_case
cargo test --manifest-path cad/wasm/Cargo.toml --locked
pnpm run build:cad
pnpm --dir cad test
```

For each isolated package after its manifest exists, set `P1_PACKAGE` and
`P1_RESULTS` to its actual task-owned paths; record values before execution:

```sh
cargo test --manifest-path "$P1_PACKAGE/Cargo.toml" --locked
cargo fmt --manifest-path "$P1_PACKAGE/Cargo.toml" --check
cargo clippy --manifest-path "$P1_PACKAGE/Cargo.toml" --locked --all-targets -- -D warnings
wasm-pack build "$P1_PACKAGE" --target web --release --out-dir worker-pkg --locked --no-default-features --features worker
```

From that package directory:

```sh
dx --version
dx build --web --release --base-path / --cargo-args="--locked"
dx build --web --release --base-path /boardstudio/ --cargo-args="--locked"
```

Tagged [target arguments](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/cli/target.rs)
verify web/release/base-path and cargo-args; installed help is checked before use.
Do not assume a worker URL/output directory: capture build output and serve
identified artifacts locally using `python3 -m http.server --bind 127.0.0.1`
with a task-owned directory/port. Define a bounded automated harness/assertions
in the task record before runtime checks; use task-owned agent-browser sessions
for browser inspection. No production origin/data or cloud browser service.

Every task retains commands, exit codes, logs, base/candidate/reviewed/integration
commits, approval/spec hashes, attempts, owned worktree and next eligible status.
Run relevant per-task checks/builds before acceptance, not occasional checkpoints.
Timing workloads are not parallelized with these builds; frozen benchmarks are
not rerun or relabeled from a transport-only probe.

## Review, repairs and stopping

Reviewers load code-review-and-quality plus actual task/spec/constraints, diff
and tests. Inspect correctness, ownership, ordering/history, compatibility,
abstraction size and weakened checks. Review the exact candidate against the
current integration base. Rebase/conflict/resolution changes require affected
verification/review again. Only coordinator integrates and validates that revision.

Allow at most two diagnosed repair attempts per failed task, retaining first
failures. Then mark it blocked; do not discard its worktree or keep investigating
indefinitely. Missing prerequisites or required checks remain blockers, not
green results. A systemic regression/uncertain compatibility blocks dependents;
unrelated authorized work may continue. Stop when no safe task in this defined
set remains, leaving a resumable handoff and verified versus unfinished results.

The probes remain isolated even if successful. Their verdict authorizes no
automatic production promotion, cutover or removal of the React reference.

Historical original P1-CORE run stopped at compatibility failure with two
exhausted repair attempts. Its renewed P1-U64 run later passed and was accepted.
The original P1-CAD continuation and failed gate are recorded in RUN/TODO; this
paragraph preserves the earlier command correction, not current task status. The command
above places wasm-pack-owned options before Cargo EXTRA_OPTIONS, matching the
observed CLI semantics. This documentation correction changes no task scope or
dependency. Reviewed failure and resumable state: [RUN](../docs/migration/RUN.md).

Current bounded follow-up: **P1-CAD-F1-r1**, explicitly approved by the user's
“do it” after the two-finding repair proposal. It preserves the old blocked run,
documents the existing experimental boundary and corrects the reference harness
only after independent cache-order controls prove contamination. The exact
task set, ownership and two-attempt limit are in
[task-start](../.scratch/prototypes/p1-cad/evidence/findings-repair/task-start.json)
and [bounded plan](../.scratch/prototypes/p1-cad/evidence/findings-repair/PLAN.md).
Existing P1 acceptance/review/integration gates still apply; P2/P3 and automatic
production promotion remain excluded.


## P2-r1 approved continuation, preflight blocked

The user's subsequent “approved” authorizes the exact
[P2-r1 lifecycle proposal](../.scratch/prototypes/p2-lifecycle/PLAN.md) from
`7ed7b5ec`: one isolated SVG/engine gesture surface and existing renderer canvas.
[Task-start](../.scratch/prototypes/p2-lifecycle/evidence/task-start.json) records
its approval/hash, specs, ownership and two-repair limit before executable edits.
This is the next accepted P2 charter, not a new architecture or migration backlog.

The required unchanged renderer fmt gate fails. Its four candidate files match
the base; no renderer change is within this task's approved file boundary.
P2 is blocked with zero repairs and no executable implementation. A concrete
formatting-only patch is preserved for one bounded scope-extension approval;
the original strict gate and all P2 runtime/build/review requirements remain.
[Current blocked ledger](../docs/migration/RUN.md#p2-r1-preflight-blocked).
