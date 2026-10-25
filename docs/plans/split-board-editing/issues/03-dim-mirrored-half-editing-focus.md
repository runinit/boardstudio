# 03: Dim mirrored-half editing focus

Status: resolved
Type: research
Map: [map.md](../map.md)

## Question

How does the mirrored half currently render and select, and what dim +
click-to-undim models fit it?

## Outcome

A survey of current mirrored-half rendering/selection plus candidate dim models
(view-only vs persisted, click semantics, per-mode scope) — enough for ticket 05
to decide. No behavior change.

## Answer

Both halves render identically, with no dim hook (`layout_canvas.rs:164`); mirror is selectable, edits translated to source (`layout.rs:314-355`), so staying linked is the norm. View-only precedent: Handles toggle. Candidate A (recommended): view-only `focused_half` workspace signal, dim non-focused half, click sets focus without document edit. Candidate B: persisted focus.

(pending)
