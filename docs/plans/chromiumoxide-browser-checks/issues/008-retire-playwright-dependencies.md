# 008: Retire Playwright browser dependencies

Status: ready-for-agent
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md), [Rust CAD runtime capture](006-rust-cad-runtime-capture.md), [Rust Graphite acceptance checks](007-rust-graphite-acceptance.md)
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

Humans and CI run every migrated real-page workflow without Playwright/npm browser setup.
Remove the obsolete runners and browser dependency installation only after every consumer's
parity gate has passed. Keep this a dependency retirement, not a new test migration.

## Acceptance

- [ ] Inventory all remaining browser consumers and runtime imports before deletion; no live
  workflow loses its runner, dependency or manual entry point. Preserve archived prototypes
  as evidence rather than deleting their historical dependency references.
- [ ] Remove obsolete Playwright runners, personal-path workarounds, browser npm install and
  NODE_PATH prerequisites from migrated workflows. Update CI caches, portable discovery,
  setup instructions and failure messages for the Rust path.
- [ ] Retain Node tooling/self-check tests still in use and their CI prerequisite. Do not
  advertise the whole repository as Node-free or touch wasm-bindgen-test/ChromeDriver.
- [ ] Clean-environment prerequisite tests and actual supported real-page checks work without
  Playwright installed or NODE_PATH set. Missing Chrome/Cargo errors remain actionable.
- [ ] Rust harness formatting/lint and the jointly pinned CI browser/library policy remain
  enforced. Document the version-bump compatibility check and upgrade path.
- [ ] Full supported verification passes on the final committed source; record any inaccessible
  CI gate explicitly rather than treating a local pass as a CI run. Existing app bugs are not
  waived and pending per-consumer parity is not grounds to retire the old dependency.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.
