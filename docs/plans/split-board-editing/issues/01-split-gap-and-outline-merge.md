# 01: Split gap and outline merge

Status: resolved
Type: research
Map: [map.md](../map.md)

## Question

Why does a large matrix plus an onboarding MCU merge both halves into one outline?
Map the current gap/axis/outline-region mechanics that cause it.

## Outcome

A written account of the split-gap storage, the mirror axis, and the outline-region
merge that collapses the two halves — enough for ticket 04 to set the adaptive rule.
No behavior change here.

## Answer

Gap default is 40mm (`mirrored_pair.rs:66`, `matrix_setup_controller.rs:81`). Geometry inset is `(gap+pitch-edge_gap)/2`; gap is key-edge to key-edge, ignoring matrix width, MCU size, and margin (`mirrored_pair_geometry.rs:116`). MCU has no auto per-half placement — the user drops it, and its courtyard can bridge the 40mm gap. Outlines: envelopes expanded by margin (`outline.rs:255-310`), islands unioned, bridges connect while shapes>1; a merged single outline means envelopes already overlapped, not a silent merge. Candidates: (a) gap >= MCU width + 2*margin + bleed; (b) gap = f(matrix width); (c) validate at creation with projected envelopes.

(pending)
