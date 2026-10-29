# 05: Provider-build parallelism

Status: ready-for-agent
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
- CI receipts come from a draft PR with user approval (outward-facing — pushing a branch alone
  triggers no CI), or from 03's probe PR once it exists.
