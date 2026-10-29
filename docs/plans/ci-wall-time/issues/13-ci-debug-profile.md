# 13: CI browser debug profile

Status: claimed
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

## Comments

- Claimed for implementation delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3Abaf24831-b75e-4a87-8cdb-b4a582ab37b1`
  (local A/B; branch `research/debug-profile-step`, no PR).
- Local A/B done 2026-10-29: `check.py` browser step now forces
  `CARGO_PROFILE_DEV_DEBUG="false"` (not `setdefault`; `check.yaml` untouched so the
  rust-cache key is undisturbed), `test-check.py` extended and 31/31 green. Catalogue: 7.7s
  vs 10.0s (+30%), 46→98.5 MB. Case: 20s vs ~24.3s (+20–25%), 73→176 MB. Cold: catalogue
  55→93s, case 115→140s. 48% figure refined down to ~20–30% warm per-launch; delta is all
  wasm serve/parse/compile, test execution identical. One line-tables case run SIGKILL-timed-out
  in the Chrome renderer after 439s under load compiling the 176 MB wasm (rerun green) — the
  `false` arm never failed. Branch `research/debug-profile-step` (`8949b519`, pushed, no PR);
  receipt in gitignored `.evidence/debug-profile-ab/`.
- Still open: the CI same-head A/B. Ordering constraint: landing this before 03 moves the warm
  baseline down by the above deltas, so 03 must score after this lands — or the CI A/B rides
  03's warmed probe.
