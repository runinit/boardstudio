# Issue 12 mounted Plate context receipt

The selected Plate layer already routes through the contextual MechanicalSettings leaf in source (`6f7aa4c1` and current `dd7697ed`). This focused paired observation records that it is present in the joined candidate so the next source packet avoids reimplementing it.

- React: profile `case-unlink-react-public-20261003`, pinned app `5173`, imported original5b SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, configured Gasket mount with current geometry. Selecting the current Plate tree row shows `Assembly settings`, `Plate`, `Plate thickness` 1.5 mm, `Resolved thickness 1.50 mm`, and the Display color/reset/visibility controls.
- Dioxus: profile `case-unlink-dioxus-reopen-34748-20261003`, candidate source `a76fa2bdee3379be1d9d2c2133f9428a87a75fb0` at `http://127.0.0.1:34748/boardstudio/`. It imported the same original5b path after the Issue14 unlink receipt, resolved current geometry, then selecting Plate showed the same contextual title, return action, 1.5 mm Plate thickness, 1.50 mm resolved thickness, and Display controls.
- React screenshot: `react-plate.png`, SHA-256 `bbe7494245bb2b55328235bdedf45f4a51e91cc768c5dff6fa1eae57deef381e`.
- Dioxus screenshot: `dioxus-plate.png`, SHA-256 `87c160e12155422e366651824432a29a51fff3492ea9182161f286b949476f0c`.

This is a mounted Plate-context observation only; Issue12's other supported layers, findings, field edits and full parent acceptance remain open.
