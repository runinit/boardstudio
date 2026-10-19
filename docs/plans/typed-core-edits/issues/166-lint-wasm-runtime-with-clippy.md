# 166: Lint the WASM Runtime with Clippy

Status: ready-for-agent
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

- [ ] The ordinary lint entrypoint invokes actual Clippy for the browser Runtime
  package's WASM production library and browser-test configurations, with explicit
  target and feature scope. It cannot be satisfied by native Clippy, typechecking
  or browser test execution alone.
- [ ] CI executes the same check using the WASM target and Clippy already declared
  by the toolchain file. Add explicit installation only if actual runner evidence
  shows it necessary. Preserve native lint, formatting and browser gates; describe
  the bounded Runtime scope without claiming whole-frontend coverage.
- [ ] Preserve locked dependency resolution, dependency-lint exclusion and warnings
  denied. Classify the first real diagnostics and make the smallest necessary
  corrections; do not add broad warning suppression or unrelated cleanup.
- [ ] Show that a temporary, deterministic Clippy violation in the WASM-only
  collision implementation fails the new check, restore it exactly, and obtain
  GREEN. Save the named Clippy diagnostic and distinguish it from syntax/type or
  tool failures. The mutation checker may label Cargo's concluding "could not
  compile" output as a compile failure; preserve that classification and report
  this separately as a lint sensitivity probe, never as a killed mutation.
- [ ] Extend the existing check-runner tests to assert the lint command's package,
  target, target kinds, locked resolution, dependency exclusion and warnings-denied
  flags, and inclusion of lint in default steps. Test CI wiring only if it changes.
  Run affected tooling checks and the standard lint entrypoint; report actual CI
  execution separately from local evidence.
- [ ] Keep Cargo launches serialized per target under the existing scheduler and
  concurrency settings. Do not broaden visibility or product behavior to make
  linting possible; report any such blocker instead.

## Comments

Created at the user's request from retro item 1 after repository and official
Cargo/Clippy research. Independent of the other three retro follow-ups.

Revised after Claude Opus audit: reuse the declared toolchain target, bound the
tooling regression to actual changes, and distinguish lint probes from mutation kills.
