# 05: Decide dim and focus interaction

Status: resolved
Type: grilling
Blocked by: [03](03-dim-mirrored-half-editing-focus.md)
Map: [map.md](../map.md)

## Question

What is the dim model? Settle view-only vs persisted dim, click semantics (select?
promote to source? just undim?), and per-mode scope (PCB/Case vs Layout only).

## Outcome

A decision on the dim model, click semantics, and mode scope. Record under the
map's Decisions so far with a link here.

Round 1 settled: Q3 dim focus recorded in map; round 2 pending.

Round 2 settled: Q9 Layout-only dimming, Q10 canvas-click-only focus recorded in map; round 3 pending.

Round 3 settled (final): Q13 recorded in map.

## Answer

- Dim is view-only focus (Handles precedent), default source half; click right half focuses/undims; no document write, no unlink (Q3).
- Dimming is Layout mode only (Q9).
- Focus changes only on canvas click of the dimmed half; tree/Inspector selection does not steal focus (Q10).
- Spacing adjust never changes focus/dim state (Q13).
