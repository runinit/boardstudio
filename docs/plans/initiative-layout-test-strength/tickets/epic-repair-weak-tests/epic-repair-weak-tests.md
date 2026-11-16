---
type: epic
title: "Repair confirmed weak tests"
parent: initiative-layout-test-strength
covers: [R1, R2, R3, R4, R5, R6, R9]
sources: []
companions: []
after: []
refined: false
risk: low
---

# Repair confirmed weak tests

## Description

Seven tests named by the audit pass whether or not the behaviour they describe works. This epic changes six of them so each fails under its counter-example, removes the seventh's redundant block, and closes by replacing the repeated fixture block with one helper per module. Parent: the initiative's Requirements R1–R6 and R9.

## Outcome

A developer who breaks case-closure adoption, a framed solve, splay-origin validation, duplicate-reference picking, rotated snapping or membership retirement sees the named test fail.

## Done when

- Each of the six repaired tests passes as delivered and fails when its counter-example is applied; the redundant duplicate-pick block is removed.
- The repeated resize/reset/zip fixture block is one helper per test module, with scenario inputs and assertions unchanged.
- A repaired test that fails against current production code has a bug ticket in this epic and lands together with its fix; no ignored test is committed.
- The full project check passes on the delivered branch.

## Boundaries

- Out: the linked key-resize test and the audit harness; owner: epic-audit-harness.
- Out: production code changes, except the fix for a bug a repaired test exposes, under its own bug ticket.
- Must not change: the behaviour each test names; existing assertions are not weakened or removed. Fixtures change and assertions may be added.

## References

- parent — docs/plans/initiative-layout-test-strength/initiative-layout-test-strength.md, section Requirements
- research — docs/plans/initiative-layout-test-strength/research-necessist-audit/research-necessist-audit.md, section Findings
