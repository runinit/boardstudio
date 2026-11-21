---
title: Reuse the patched Wasm tools across unrelated workspace changes
type: chore
ticket: 1
created: '2026-11-17'
status: done
baseline_revision: '95e7091c45065f174a62c682eeeae658f4e46b30'
route: full
route_source: auto
risk: medium
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

Both web jobs can rebuild the same patched tools after an unrelated workspace package changes version. Give the jobs a shared key based on the inputs that built the pair. Check workspace compatibility before lookup and run both binaries before passing them to either suite.

## Boundaries & Constraints

Preserve the packaged and Wasm matrix entries, dev-specific checks, patched tool selection, browser isolation and release gating. Work locally. Hosted cache miss frequency and workflow time savings are unknown.

## I/O & Edge-Case Matrix

| Given | When | Then |
|---|---|---|
| A compatible workspace and an empty cache | A consumer requests the tools | The builder produces both pinned executables. |
| A populated cache | An unrelated workspace package changes version | Both consumers compute the original key, verify the pair and skip the builder. |
| A prospective cache hit | The lock has zero, multiple or incompatible wasm-bindgen entries | Validation fails before restore or suite execution. |
| A known key | Any one build input changes independently | The key changes. |
| A cache hit | Either executable is absent, fails to execute or reports an unexpected name or version | Verification fails before the consumer runs. |

</frozen-after-approval>

## Code Map

- `scripts/build-patched-wasm-bindgen.py` owns the archive checksums, tiny_http patch, upstream lockfile adjustment, pinned toolchain selection, Cargo command and executable names. Retain the direct build invocation and atomic pair installation.
- `.github/workflows/check.yaml` configures the web matrix and its cache steps. Limit changes to patched tool setup and retain dev-specific commands and environments.
- `scripts/test-build-patched-wasm-bindgen.py` supplies local archive and Cargo fixtures. `scripts/test-ci-workflow.py` checks the matrix, release gate, cache policy and toolchain order.

## Tasks & Acceptance

- [x] In `scripts/build-patched-wasm-bindgen.py`, parse the workspace lock as TOML and require exactly one wasm-bindgen package at the pinned version. Run the check for direct builds and on the path to every lookup. Reuse the existing exact name/version checks for restored executables.
- [x] In the builder and `.github/workflows/check.yaml`, compute the same versioned key for both consumers. Map the patch text, source checksums and pins, upstream dependency lock and build recipe to the builder and archive identities. Include the effective compiler identity, target OS and architecture, toolchain file and relevant Cargo configuration. Document each input beside the key or builder.
- [x] In `.github/workflows/check.yaml`, restore by exact key, invoke the builder only on a miss, then verify the directory supplied to both browser consumers. Preserve the packaged build, Wasm suite and existing dev-specific coverage.
- [x] Extend the two Python suites with named controls for all matrix rows. Vary build inputs one at a time, count builder calls, and exercise failures separately for each binary. Check the actual workflow ordering, key, paths and conditions so a disconnected local helper cannot satisfy the workflow test.

Given either consumer, when its local hit and miss controls run, then the observed builder calls and tool selections match the matrix. Given the existing dev checks, when the workflow suite runs, then its original assertions pass alongside the new controls.

## Implementation Notes

Eight builder tests and sixteen workflow tests passed locally, including cross-suite reuse, independent invalidation, incompatible locks and binary failures. A fresh production build produced and verified both executables at `0.2.129`.

## Plan Change Log

## Review Triage Log

Quick review raised four findings. Three medium findings were patched, one was rejected, and none were deferred.

| Finding | Verdict and evidence |
|---|---|
| Cargo environment overrides | Medium, patched. Key generation probes `CARGO_BUILD_RUSTC` when `RUSTC` is unset and fingerprints compiler, flags, target and release build overrides. |
| Prefix fallback could occupy the build directory | False for this workflow's caches. Every v1 key has the same prefix and exactly 64 hex digits. No generated key is a proper prefix of another and the older namespace differs. |
| Workflow could discard the builder key | Medium, patched. The emitted workflow output changes independently for a flags change and for a different effective compiler at the same path. |
| Fixture stopped before consumer tool selection | Medium, patched. Both hit and miss paths exercise production browser plan environment configuration and inspect both resolved paths at command launch. Separate Wasm runner checks cover page and direct crate launches. |

After the patches, all 8 builder and 17 workflow tests passed locally. A fresh production build passed `--verify` and both tools reported `0.2.129`. The Wasm runner suite passed 59 tests. Hosted cache behavior and wall-time savings remain unmeasured.

## Design Notes

The CLI archive supplies its own Cargo.lock and the builder uses `--locked`. The verified tiny_http archive provides the patched source. The workspace lock supplies the required wasm-bindgen version for compatibility checking. Unrelated workspace entries do not participate in the CLI build.

## Verification

- Run `python3 scripts/test-build-patched-wasm-bindgen.py` and `python3 scripts/test-ci-workflow.py` from the repository root. Both suites must pass and exercise the named controls.
- Run the production builder into a fresh local output directory, then verify both executables. Label any elapsed time as a local observation.
