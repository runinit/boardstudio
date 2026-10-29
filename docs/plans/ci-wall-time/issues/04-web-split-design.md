# 04: Web-split design

Status: ready-for-human
Type: grilling
Blocked by: [03: Warm-baseline measurement](03-warm-baseline-measure.md), [09: Implement tiered PR gate](09-tiered-pr-gate.md)
Parent: [CI wall time map](../map.md)
Related: [13: Browser structural](../../check-speed/issues/13-browser-structural.md)

## Question

How should the single `web` job (build plus browser serial) split into parallel jobs — which
shards, which artifacts are shared, and what does the transfer cost?

## Pointers

- Needs 03's warm numbers first: the compile-versus-test split decides whether sharding the
  crate batches (cold: 432s / 264s / ~226s serial) beats sharing one build. Waits on 09 as well:
  a smaller PR tier changes the shard design — and may make this ticket unnecessary (close it
  with a note if so).
- Sketch: build once, upload providers/site artifact, shard crate batches plus presentation
  across runners; each shard still needs a cache restore.
- Close to check-speed 13 option A (one cargo build) but scoped differently: 13 targets local
  tooling, this targets CI runners — that distinction is why it lives here.
- This is a human-in-the-loop decision with the user once the numbers exist.
