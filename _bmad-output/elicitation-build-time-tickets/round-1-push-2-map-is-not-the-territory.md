Map Is Not the Territory: The backlog describes cache reuse as preceding the scheduling experiment, but its dependency metadata and baseline do not consistently encode that sequence. Trust the settled sequence and the actual revision under test.

## Findings

1. `_bmad-output/backlog/spike-packaged-build-overlap.md:6` and `:16` — `after: []` leaves the cache story unordered even though the description says to schedule it first. Set `after` to the cache story’s ticket identifier, using the backlog’s supported dependency format.
2. `_bmad-output/backlog/spike-packaged-build-overlap.md:20` — Fixing the experiment to `a6cfa5183` would test the pre-cache baseline even after the cache story ships. Require the local revision containing the completed cache change, record its commit, and use that same revision for both schedules; retain `a6cfa5183` as the original observation baseline.
