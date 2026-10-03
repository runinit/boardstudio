# F3.5a component layout selection survives tab changes

This focused receipt covers the mounted Layout component Inspector in the
same imported Sofle fixture on pinned React (`5a472a9426e6e38993361da402cd4ec730feb369`)
and frozen Dioxus candidate (`fe86fa05598dee6ef8ffb02f9f6fcfaa6810f10d`, served
at `http://127.0.0.1:34750/boardstudio/`, provider provenance
`3ee7a6c10ba260d41233d5fc1cd1b7f1f518b0bc6b6f029bacae553a59f74740`).

Fixture: `layered-sofle-export.boardstudio`, SHA-256
`5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
Select: Part → `left-U1` on the left board. Its initial Component layout is
`left-keys-layout` / `keys` in both frontends.

RED: on Dioxus, Properties → Relations → Properties clears the native select's
selected option and resets it to Board / ungrouped (`value=""`). The accepted
projection remains `left-keys-layout`; this is a rendered control-state defect.
On pinned React, the same tab sequence retains `left-keys-layout` / `keys`.
The observed post-tab values were read from the mounted `[aria-label="Component
layout"]` element in named independent browser sessions.

Repair: each native option now renders an explicit selected state from the
accepted `projection.layout_id`, so remounting Properties restores the current
membership without changing the accepted document or edit path.

GREEN: pending the next integrated candidate. Required focused check is the same
mounted Properties → Relations → Properties journey on that candidate; no helper
test substitutes for it. This repair adds no history entry and leaves the
existing membership edit owner unchanged. No new refactoring takeaway observed.
