# Guarded page-only packaging independent review

Exact source `d90c0d453ea45fc09d11a74c1a60a050b8bc3325`, isolated `packaging-page-reuse-20261002`, base `69159aa70a0f2f661abfc08a1ea4a6f7a787098d`. Frozen diff contains only `scripts/build-m1.py` and `scripts/test-build-m1-reuse.py`. Reviewed against independently approved `.scratch/dioxus-frontend-v1/specs/page-only-packaging-reuse.md` and current CONSTRAINTS. Requested Sol6.1 High independent both-axis review; no additional subagents, application edits or real build.

Disposition: **HOLD packaging source clearance for the two Spec findings below.** Existing passing stub checks remain valid for what they execute, not sufficient to prove these omitted cases.

## Standards

No separate blocking documented-standards finding. CLI validation uses explicit errors and the new tests preserve ordinary assertions under optimized production Python. The author retained the full22 path, unique directories and original build features rather than silently falling back. No public/API widening, lowered thresholds, skips or checker suppression found.

Possible Duplicated Code (judgment, nonblocking): the complete22 command shape is independently maintained in `expected_full_commands` (136–164), `build_full` (476–508), and fixture `full_argv` (114–154). A future legitimate command change can make valid full baselines unusable or tests agree with the wrong expectation. Later extract one production command specification while retaining behavior-focused fixture assertions. This is an existing RF-001/RF-009 maintenance observation, not authority to expand this repair into a generic build framework.

## Spec

### P1 — Root build configuration is invisible to source identity

`scripts/build-m1.py:79–82` includes selected tree prefixes and only four root files. `.cargo/config.toml`, legacy `.cargo/config`, `.npmrc` and `.node-version` are omitted even when Git lists them. Root Cargo config is consumed by Rust builds: adding/changing rustflags can alter the fresh page/offline binaries while reused providers remain compiled with the old configuration, with all tool versions equal. This violates the contract's “all dependency/config/generator/tool inputs” and rejection of additions/changes to consumed inputs before side effects.

Actual disposable `sources()` reproduction created `web/src/main.rs`, root `.cargo/config.toml`, `.npmrc`, and `.node-version` and returned all paths from the Git enumeration. Inventory contained only `web/src/main.rs`. Mutating Cargo config to `[build] rustflags=["--cfg", "changed_provider"]` left the manifest identical. Include relevant root build configuration and add/change/delete tests using the actual inventory; preserve explicit output exclusions.

### P2 — Candidate reused-provider path sets are never compared

`scripts/build-m1.py:412–421` iterates only the baseline's reused-provider keys and compares their candidate hashes. Additional files under the reused prefixes are admitted. This violates the contract's completion comparison of “reused artifact path sets/hashes” and immutable reused-tree identity.

Actual disposable staging reproduction wrapped the production `copytree(public, destination, dirs_exist_ok=True)` to add `assets/core-worker/additional.js` after fresh page staging. Both route candidates retained that extra provider file; the existing eight-command fixture passed and recorded `status: complete`. Compare the complete candidate provider-prefix map to the baseline map, including equal path sets and bytes. Add a during-staging extra-file regression. Construct fresh route sites from validated reused provider/staged assets plus fresh page/generator/offline outputs; copying the full baseline site at390 and deleting only known page names requires explicit complete classification so stale page outputs cannot survive unclassified.

## Executed verification and remaining gates

Independently executed at the frozen SHA:

- `python3 scripts/test-build-m1-reuse.py`:11 passed, disposable/stub builds only.
- `python3 scripts/test-build-m1-sources.py`:1 passed.
- `python3 -m py_compile scripts/build-m1.py scripts/test-build-m1-reuse.py`:passed.
- `git diff --check 69159aa7..d90c0d45`:passed; clean exact HEAD before checks.
- Two additional disposable negative reproductions above demonstrate omitted protections; no application or script source was modified.

Author prior expected-red evidence for the CLI --help defect is retained: the old script created an output and invoked a rustc sentinel; the new real CLI fixture has no executor/output side effects. Optimized-Python coverage currently runs only invalid-ID validation; it does not execute all source/artifact guards under `python -O`. The11 tests also do not execute the original full22 pipeline fixture. Keep the contract's broader fixture coverage and real helper-matched full22 baseline plus one eligible reuse candidate/root/subpath/offline browser evidence open. Current Case/Firmware and any pre-helper baseline remain ineligible. No measured timing saving or parent acceptance follows.

Authors/coordinator notified of both holds. No new RF ID; retain RF-001/RF-009 for source/evidence/build ownership and exact provenance. No deferred correctness waiver.
