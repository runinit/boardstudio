# Baseline

Baseline observations for this effort. Hosted job walls include setup and cache restore on
GitHub-hosted Ubuntu runners with 4 vCPUs; the local observation below is separate.

## Local web baseline — 2026-10-29

One unsharded `python3 scripts/check.py build browser` run passed on the clean experimental
worktree at `feat/dependency-aware-web-execution@932bce6f266b395fc6bf3649a14587ad261f08f9`:
746.5s wall, with reported build 299.6s and browser 446.6s. It reported 13 drag checks, 364 CAD
browser checks, 14 crate launches with 587 passing tests, and 77/77 page tests across the three
existing isolated filters, with no incomplete or duplicate results reported. Workspace and CAD
targets were initially absent; Cargo registry/Git and wasm-pack caches were populated. Cargo jobs
were 12 and the scheduler used 4; CI's `playwright-core@1.62.1` was installed in an isolated tools
directory. Sources: [baseline log](/home/chris/.t3/worktrees/ts-boardstudio2/t3-897cd35c/.evidence/web-baseline-932bce6f-wyru0xk7.log)
(including the drag receipt) and [page results](/home/chris/.t3/worktrees/ts-boardstudio2/t3-897cd35c/web/target/test-results/browser.json).
These are local, gitignored artifacts and may not be available elsewhere. This single run alone is
not a same-head split comparison, hosted CI speed claim, or fully cold-cache full-gate result. A
later [local comparison](../issues/04-web-split-design.md#local-same-head-split-parity--2026-10-29)
found same-head test-result parity, subject to its stated limitations. See
[04](../issues/04-web-split-design.md) and [16](../issues/16-web-partition-proof.md).

## Hosted and other anchors

- On 2026-10-29, run [40791505198](https://github.com/runinit/boardstudio/actions/runs/40791505198)
  on head `d650e1ddf` went fully green: `native` 30m19s (`repo` 3.5s, `tooling` 143s, `lint`
  477s, `test` 1095s), `web` 33m37s (`build` 624.3s, `browser` 1363.4s). Both jobs ran fully
  cold: rust-cache "No cache found", wasm-pack "Cache not found". Per-command receipt:
  `.evidence/web-ci-green-baseline-d650e1dd/receipt.md` (gitignored, main checkout only).
- The previous red run 40776517605 failed at 47m02s; red-run timeouts and the
  panic-hook cascade inflated the wall by about 13.5 minutes versus green.
- Warm anchors were PR 7's web 22m52s with a dev-cache restore (run 40730153041), same-head
  only for direction. Scored 2026-10-29 on draft PR #11 (probe head `008cb78f` = `d650e1ddf`
  plus five cache commits): cold native 29m22s / web 42m17s (transition, red cache), warm
  native **14m41s** / web **26m03s** (same head, green). Tests identical: 2870 native passed,
  364 browser checks, 659 wasm unit, 5/5 drag-smoke. Two same-head reds on the way (0.2s
  localhost timeout; drag-smoke translate diff) — flakes, receipt stands.
- Cold build split (web job): `dx` page ~311s, core-worker ~162s, cad-wasm ~99s, rest ~50s.
  Cold browser split: crate batches ~432s / ~264s / ~226s serial, page `--list` ~36s,
  `run-wasm-tests --all` ~314s for 107 tests.
- On 2026-10-29, [ticket 05](../issues/05-provider-parallelism.md) reported local cold
  **provider-phase**, not end-to-end, walls on `research/provider-parallelism` (`8bf8ebf63`):
  `--fast` 150.3 → 133.6s and shipped 433.3 → 355.7s. The local build branch
  `research/local-web-builds` at `0f7c63087b6b488a5ad340e0e31ed9c135f8fd7d` reuses that
  implementation and added focused scheduling tests (10/10 reported passing). No fresh
  end-to-end before/after timing was reported for that branch; no new build timing was taken
  while tuning was active. Do not equate `--fast` with a shipped build.
- On 2026-10-29, the repeat-`wasm-opt` investigation reported no safe trivial unchanged-input
  skip. Its isolated mocked check on `research/local-web-builds` at `0f7c63087` found the CAD
  build still skipped when `RUSTFLAGS` changed and its JS output was missing. The source at that
  revision, `cad/scripts/cad_build.py` (`wasm_inputs_digest` and `build_wasm`), hashes selected
  inputs but checks only the WASM output and stamp before skipping. This is an existing
  stale-output risk, not a safe cache pattern to copy; the check was mocked, not a build timing.
- On 2026-10-29, `research/cad-build-stamp-safety` at `8d6bbd0fe` removed that early skip,
  invalidates the stamp before building, and checks both WASM and JS outputs; the commit includes
  regression tests. This is a local research-branch fix, not a claim it landed on `dev` or in CI.
- On 2026-10-29, the native CAD shared-target experiment at `8d6bbd0fe` observed 59.1s versus
  68.0s for one WASM native-test comparison, with the expensive Monstertruck stack still
  recompiling. This is one noisy local observation, not a full-gate saving. Logs, conditions,
  uncovered freshness and qualification cases are recorded in [17](../issues/17-native-topology.md).
