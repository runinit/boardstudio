# 05: Provider-build parallelism

Status: claimed
Type: task
Blocked by: [02: Provider-build overlap](02-provider-overlap.md)
Parent: [CI wall time map](../map.md)

## Question

Implement overlapping provider builds and prove the gain with a same-head A/B plus a green full
gate.

## Pointers

- Proceed on 02's go-ahead: three lanes (A serial wasm-pack, B CAD-WASM, C ergogen), no
  per-lane `CARGO_TARGET_DIR` isolation; run 02's specified H1/H2 test (does the `dx` compile
  read staged `web/assets`?) and cold re-measure both `--fast` and shipped paths.
  Full design: `docs/research/2026-10-29-provider-overlap.md` on `research/provider-overlap`.
- Saving estimate is ~90–110s of the 624s cold build (02's verdict); re-measure, don't claim
  it. Warm-run savings shrink with caches and must be reported separately.
- Acceptance: before/after walls on the same head, test counts unchanged, green full gate.
  `BOARDSTUDIO_CHECK_JOBS` interplay and target-dir contention must be in the receipt.

## Comments

- Claimed for implementation delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3Aa9cea27e-50b2-49d7-9c11-6150f2e42c9d`
  (three lanes + H1/H2 + local A/B; branch `research/provider-parallelism`, no PR;
  CI receipt queued behind a later draft-PR approval).
- CI receipts come from a draft PR with user approval (outward-facing — pushing a branch alone
  triggers no CI), or from 03's probe PR once it exists.
