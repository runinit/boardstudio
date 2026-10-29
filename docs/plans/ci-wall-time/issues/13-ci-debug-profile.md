# 13: CI browser debug profile

Status: ready-for-agent
Type: task
Parent: [CI wall time map](../map.md)
Related: [12: Browser quick wins](../../check-speed/issues/12-browser-quick-wins.md)

## Question

Set the CI browser debug profile to `false` and prove the gain with a same-head A/B — this plan
owns the item because check-speed excludes CI changes.

## Pointers

- Mechanism: the workflow sets `CARGO_PROFILE_DEV_DEBUG: line-tables-only` globally while the
  browser step only `setdefault`s `"false"`, so on CI the test wasm keeps line tables.
- Check-speed 04 measured line tables ~48% slower per launch at twice the size (local); confirm
  on CI, where the cold browser step is 1363s.
- Cross-link check-speed 12:19 (lists the candidate); this ticket produces this plan's own
  evidence with the standard receipt (same head, test counts, green gate).
- Change point matters: Swatinem rust-cache folds `CARGO_*` env vars into its key, so editing
  the job-level `CARGO_PROFILE_DEV_DEBUG` would cold-miss the "after" arm. Change the browser
  step to force `"false"` instead of `setdefault`, or set the variable at step level — and
  require both arms at equal cache state (both warm, or one cold pair plus one warm pair).
- Ordering: if this lands before 03 scores the baseline, the warm baseline moves — say which
  baseline the A/B compares against.
