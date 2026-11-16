---
type: epic
title: "Audit harness"
parent: initiative-layout-test-strength
covers: [R7, R8]
sources: []
companions: []
after: []
refined: false
risk: low
---

# Audit harness

## Description

One required test never runs because it needs an external fixture, and Necessist cannot see the Core integration tests at all. This epic gets linked key resize covered in the ordinary gate and proves, with a pilot audit of `core/tests/matrix_placement.rs`, whether a Necessist audit of those tests is trustworthy and worth its cost. Parent: the initiative's Requirements R7 and R8.

## Outcome

The team can decide from measured results, not estimates, whether to audit the remaining Core integration tests before any Layout engine refactor.

## Done when

- No ignored linked key-resize test remains: existing coverage is shown and the test deleted, or it is rebuilt on a synthetic document constructed in test code; linked key resize through Core passes in the full project check.
- `core/tests/matrix_placement.rs` has an audit in which the named tests are shown to run and a deliberate failing control is detected.
- Each finding from that audit reproduces in the normal `integration` target.
- A written result in this epic's Notes records removals per hour, findings, and a recommendation on a wider audit.
- The full project check passes on the delivered branch.

## Boundaries

- Out: repairing the seven named tests; owner: epic-repair-weak-tests.
- Out: auditing any other Core integration file.
- Out: restructuring the tracked Core integration tests.
- Out: delivering audit scaffolding; temporary `[[test]]` entries or a scratch crate are never committed, and neither is any Necessist database.
- Must not change: how the `integration` binary is built and run by the project check.
- The pilot audit runs at a recorded commit in its own worktree with its own database, because Necessist rewrites test files in place while it runs.

## References

- parent — docs/plans/initiative-layout-test-strength/initiative-layout-test-strength.md, section Requirements
- research — docs/plans/initiative-layout-test-strength/research-necessist-audit/research-necessist-audit.md, sections Lead, Harness limitation, Review corrections
