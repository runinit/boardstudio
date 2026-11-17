---
id: 5
type: bug
title: "Dependency audit blocks hosted CI on zip 9.0.1"
parent: ""
covers: []
after: []
assignee: ""
refined: true
hitl: false
risk: low
severity: P1
---

# Dependency audit blocks hosted CI on zip 9.0.1

## Description

The dependency audit blocks the hosted CI trials for PRs #23 and #24. The audit should succeed against their shared base while Core archive behavior remains compatible on native and WASM targets.

## Reproduction

At `46d758d3fe8fc049084c87f077083dbe3eae137b`, run `python3 scripts/check.py security` with `cargo-audit` installed. It reports `RUSTSEC-2026-0548` for `zip` 9.0.1 and exits 1, matching the hosted job.

## Cause Hypothesis

Core pins the affected release, so the workspace lockfile resolves that release for the repository audit.

## Acceptance Criteria

1. Given the corrected workspace, when `python3 scripts/check.py security` runs with the advisory database, then it exits 0 without reporting `RUSTSEC-2026-0548` or adding an advisory exception.
2. Given the resolved dependency, when `cargo test --locked -p boardstudio_core --lib archive::`, `cargo test --locked -p boardstudio_core --test integration legacy_archive_migration::`, and `cargo check --locked -p boardstudio_core --target wasm32-unknown-unknown` run, then all three pass.
3. Given the shared base of PRs #23 and #24, when the draft delivery is compared to that base, then the diff contains only the dependency fix and this bug's BMad records.
4. Given evidence that the reproduction already passes on the shared base, when the cause is checked against that evidence, then the plan records the result and the dependency change is unnecessary.

## References

- Hosted audit job — https://github.com/runinit/boardstudio/actions/runs/44157335345/job/134301488068

## Notes

- Risk is low because the change is confined to the affected dependency and its workspace resolution, with existing archive coverage.
- The delivery base is `audit/layout-engine-pcb-layout`. The trial PRs and the CI progress epic are outside this ticket.
