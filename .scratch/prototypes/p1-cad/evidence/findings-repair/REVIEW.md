# Exact P1-CAD-F1 source review

Candidate8015b57f against90d85715. Runtime routing in review-routing.json; snapshot taken before final reviewer archival.

## Standards

Standards review: 0 documented violations; 0 heuristic smells. Exact candidate: 8015b57f7d149ceb8d14aa5c5adff40bb18deab5, reviewed against 90d857156fae5f8279840763b10d8571090b8724 (one commit). The specified checkout was clean at that HEAD.

The harness retains the preview and export vertex-count assertions and adds byte-for-byte position and normal checks for matching lifetimes. The architecture entry covers owners, data and buffer flow, cancellation, errors, cleanup, measured transfer costs, and retirement criteria. The README clearly labels the original run as blocked and unaccepted. The recorded final native, CAD, release, and browser gates pass; I did not rerun them.

One wording caveat: docs/architecture.md says geometry, material, bounds, and STEP oracles remain strict, while this repair’s recorded STEP oracle checks solids, volume, and bounds; it records no material assertion. I read that sentence as preserving broader project oracles, not claiming this fixture tested material.

No edits, builds, or tests were performed.

## Spec

Spec review: no material findings. Reviewed the exact candidate 8015b57f7d149ceb8d14aa5c5adff40bb18deab5 against base 90d857156fae5f8279840763b10d8571090b8724; HEAD matched and the worktree was clean.

The approved two-finding scope is addressed. The architecture entry documents boundary ownership, data and buffer handling, errors, cancellation, cleanup, measured transfer costs, and retirement criteria. The repaired oracle keeps exact assertions while comparing matching cache lifetimes; the original failed assertion and exhausted run remain preserved. I found no scope creep beyond the approved prototype, evidence, and status/documentation changes.

The recorded evidence includes 3 prototype tests, 23 native CAD tests with 4 existing ignored benchmarks, 49 JavaScript regressions with none skipped, fresh release builds and Chromium checks at both prefixes, byte-exact mesh comparisons, and independent STEP checks across the cache histories. I inspected the Rust host, worker, app, protocol tests, harness, reports, and compressed logs. I did not rerun checks or modify files.

This review does not certify P2/P3, full M1, performance, broader geometry/platform coverage, or full-range CAD revisions; the task correctly records those limits.

Standards:0 material findings; Spec:0 material findings. No material worst issue on either axis.
