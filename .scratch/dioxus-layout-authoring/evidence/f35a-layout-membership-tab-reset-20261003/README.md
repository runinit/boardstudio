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

GREEN: integrated candidate source
`243aa55108fab09b35e6f2e1fcfa6fbb92e5ef53`, served at
`http://127.0.0.1:34755/boardstudio/` with provider provenance
`aedb689f011b9f4a8955b76fcbfd72e988665cb8435b1858f91276237c5e56bc`. The same
fixture was used in named React and Dioxus browser sessions. On both, selecting
`left-U1` starts at `left-keys-layout`; Properties → Relations → Properties
leaves it at `left-keys-layout` with `keys` selected.

For the constraint draft leg, on both apps the mounted form was set to Mirror,
source `left/SW25`, Horizontal without saving. Returning through Relations
restored the accepted default state: the constraint section is closed; opening
it shows Offset and the first board part (`matrix/left-keys/r0c0`). This matches
React's remounted Properties editor and removes Dioxus's prior mixed-state case.
No accepted edit was submitted in this journey. Screenshots:
`react-final-tab-return.png` and `dioxus-final-tab-return.png`.

The same return journey exposed a second draft-state discrepancy. With no
accepted constraint, choose Mirror / source `left/SW25` / Horizontal without
saving, then switch Relations → Properties. Pinned React resets this draft to
the accepted default Offset / first source when its Properties editor remounts.
On candidate `b3c6909b`, Dioxus kept the Mirror and Horizontal signals but its
native source select displayed the first source `matrix/left-keys/r0c0`; saving
could therefore submit a different source than the displayed draft. The repair
resets all local Properties drafts to accepted values when returning from
Relations, and renders explicit selected option state so accepted non-default
constraint values rehydrate accurately. See `dioxus-constraint-source-reset-red.png`
and `react-constraint-tab-return.png`.

The mounted membership and draft-reset journey is GREEN. This does not claim
issue12's full accepted-edit, Undo/Redo, navigation, validation, or save/reopen
acceptance. No new refactoring takeaway observed.
