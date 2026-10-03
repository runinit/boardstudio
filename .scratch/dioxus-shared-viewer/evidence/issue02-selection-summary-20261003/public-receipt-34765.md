# Case selection context presentation on 34765

This bounded public-browser journey used the original layered Sofle archive `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, imported with the visible picker in the task-owned browser session. React reference was pinned at `5a472a9426e6e38993361da402cd4ec730feb369`. Dioxus candidate was `34765`, source `17ba32b984c13f3621a2145efe1dd478862ab5cd`, provenance `273ca2fa2315b32fec91b0490d2f70fb03b5426d759310bced6e934ad73947d5`.

The existing decoded native preview reported `90 of 90 board models decoded`. After expanding the PCB object branch and fitting the assembly, clicking the visible model at `(618,313)` selected the exact current `left-keys-SW2` tree row and kept the Case workspace active. The Inspector preamble read `Left PCB / Case / keys`; the Objects footer read `left-keys-SW2`, the same breadcrumb, and `1 selected`; and the canvas badge read `left-keys-SW2`. This matches the retained React pick capture at `34763`, including the separate breadcrumb, footer and canvas placements.

Clicking `Left case assembly` cleared the PCB part selection and restored the unselected presentation: the footer showed `Left PCB`, `Case`, and `Select an object to edit`; the generic physical-assembly prompt returned; and the canvas badge disappeared. Both changes were selection-only; no project edit was made.

Captures:

- Before pick: [`34765-before-pick.png`](34765-before-pick.png)
- Selected: [`34765-selected.png`](34765-selected.png)
- Cleared: [`34765-unselected.png`](34765-unselected.png)
- Paired pinned React reference: [`34763-react-case-picked-part.png`](../issue02-case-pick-next-20261003/34763-react-case-picked-part.png)

This receipt qualifies only the confirmed current-selection presentation delta. It does not expand mapped-pick model-family coverage, empty/stale pick coverage, or the broader lifecycle/accessibility matrix; those remain with Issue02/F7.3. No new refactoring takeaway was observed; preserve RF-002/RF-012 unchanged.
