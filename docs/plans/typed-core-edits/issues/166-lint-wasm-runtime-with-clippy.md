# 166: Lint the WASM Runtime with Clippy

Status: resolved
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [WASM lint research](../../../research/retro-wasm-clippy.md)

## What to build

Make the standard local and CI checks run Clippy on the browser Runtime's WASM
production and test configurations. Native Clippy currently sees a different
Runtime implementation, so a passing native lint gate does not establish lint
coverage for the complete collision-check module.

This is a bounded first slice for the Runtime package, not a claim that every
browser package is linted. Use the existing check pipeline and its build scheduling
rather than creating an independent competing build lane. Research identifies a
candidate package/target selection but has not executed it or established whether
existing WASM warnings need correction.

## Acceptance criteria

- [x] The ordinary lint entrypoint invokes actual Clippy for the browser Runtime
  package's WASM production library and browser-test configurations, with explicit
  target and feature scope. It cannot be satisfied by native Clippy, typechecking
  or browser test execution alone.
- [x] CI executes the same check using the WASM target and Clippy already declared
  by the toolchain file. Add explicit installation only if actual runner evidence
  shows it necessary. Preserve native lint, formatting and browser gates; describe
  the bounded Runtime scope without claiming whole-frontend coverage.
- [x] Preserve locked dependency resolution, dependency-lint exclusion and warnings
  denied. Classify the first real diagnostics and make the smallest necessary
  corrections; do not add broad warning suppression or unrelated cleanup.
- [x] Show that a temporary, deterministic Clippy violation in the WASM-only
  collision implementation fails the new check, restore it exactly, and obtain
  GREEN. Save the named Clippy diagnostic and distinguish it from syntax/type or
  tool failures. The mutation checker may label Cargo's concluding "could not
  compile" output as a compile failure; preserve that classification and report
  this separately as a lint sensitivity probe, never as a killed mutation.
- [x] Extend the existing check-runner tests to assert the lint command's package,
  target, target kinds, locked resolution, dependency exclusion and warnings-denied
  flags, and inclusion of lint in default steps. Test CI wiring only if it changes.
  Run affected tooling checks and the standard lint entrypoint; report actual CI
  execution separately from local evidence.
- [x] Keep Cargo launches serialized per target under the existing scheduler and
  concurrency settings. Do not broaden visibility or product behavior to make
  linting possible; report any such blocker instead.

## Comments

Created at the user's request from retro item 1 after repository and official
Cargo/Clippy research. Independent of the other three retro follow-ups.

Revised after Claude Opus audit: reuse the declared toolchain target, bound the
tooling regression to actual changes, and distinguish lint probes from mutation kills.

## Answer — 2026-10-18

Implemented in `b4a4595b`, with review corrections in `4a49af46`. The standard lint
step retains formatting/native Clippy and adds the bounded WASM Runtime library and
browser-test configurations. The new command-selection regression failed because
the WASM command was absent, then passed; all 31 runner and 15 quick-runner tests
pass. No broad lint suppression, feature widening or Rust behavior change was needed.

Actual WASM Clippy passed without warnings. A temporary identity operation in the
WASM-only collision module produced `clippy::identity_op` for library and library-test
builds. The project mutation checker reported `compile-failure (does not count)`;
this is lint sensitivity evidence, not a killed mutation. Exact restoration was
verified and the restored command and standard lint entrypoint passed.

At clean code revision `4a49af46`, the full six-gate check passed in **526.6 s**.
CI already invokes the same lint step and the toolchain declares its target/component;
remote CI was not run. Coverage remains Runtime-only. Evidence:
`.evidence/retro-implementation/166/` and `full-check.log` under that effort's parent.
The two-axis DeepSeek review findings were corrected and verified before the full run.
