# Baseline

The numbers every receipt in this effort compares against. Walls are job walls including setup
and cache restore, from GitHub-hosted Ubuntu runners with 4 vCPUs.

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
