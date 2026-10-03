# Ready-row status dot: rebuilt-package visual recheck

This narrow check verifies the reviewed success-surface token correction in the rebuilt root package. It does not close the Export parent or exercise the not-ready/error row.

## Inputs

- React oracle: pinned source `5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5173/`.
- Dioxus candidate: source `d7ff5e3dcae67a719caf6660cfcff57c3811df34`, build `frontend-ready-export-theme-reuse-20261002`, `http://127.0.0.1:34736/` and `/boardstudio/`; root-reported provenance SHA-256 `60f51e4b5775b1541131d661046fb9b751f77d812569eb366018c94f70640fe9`.
- Fresh isolated browser sessions: `ready-dot-react-20261002` and `ready-dot-dioxus-20261002` (Headless Chrome 154, viewport 1280×577, DPR 1).
- Both profiles imported `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## Result

The Export workspace showed an enabled `Export ZMK firmware` action in both applications. Computed ready-dot colors exactly match per theme:

| Theme | React `.wb-ready-dot.is-ready` | Dioxus `.m1-export-ready-dot.is-ready` | Shared success token |
| --- | --- | --- | --- |
| Light | `rgb(226, 242, 233)` | `rgb(226, 242, 233)` | `#e2f2e9` |
| Dark | `rgb(24, 53, 45)` | `rgb(24, 53, 45)` | `#18352d` |

Screenshots and their SHA-256 hashes are retained in `react/` and `dioxus/`. The prior accent-colored dot is no longer present in this rebuilt package. The status dot is presentation-only; this check does not test firmware generation, download, or failure behavior.
