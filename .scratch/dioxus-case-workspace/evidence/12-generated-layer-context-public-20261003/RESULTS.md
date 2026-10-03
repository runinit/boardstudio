# Issue 12 mounted Plate context receipt

The selected Plate layer already routes through the contextual MechanicalSettings leaf in source (`6f7aa4c1` and current `dd7697ed`). This focused paired observation records that it is present in the joined candidate so the next source packet avoids reimplementing it.

- React: profile `case-unlink-react-public-20261003`, pinned app `5173`, imported original5b SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, configured Gasket mount with current geometry. Selecting the current Plate tree row shows `Assembly settings`, `Plate`, `Plate thickness` 1.5 mm, `Resolved thickness 1.50 mm`, and the Display color/reset/visibility controls.
- Dioxus: profile `case-unlink-dioxus-reopen-34748-20261003`, candidate source `a76fa2bdee3379be1d9d2c2133f9428a87a75fb0` at `http://127.0.0.1:34748/boardstudio/`. It imported the same original5b path after the Issue14 unlink receipt, resolved current geometry, then selecting Plate showed the same contextual title, return action, 1.5 mm Plate thickness, 1.50 mm resolved thickness, and Display controls.
- React screenshot: `react-plate.png`, SHA-256 `bbe7494245bb2b55328235bdedf45f4a51e91cc768c5dff6fa1eae57deef381e`.
- Dioxus screenshot: `dioxus-plate.png`, SHA-256 `87c160e12155422e366651824432a29a51fff3492ea9182161f286b949476f0c`.

This is a mounted Plate-context observation only; Issue12's other supported layers, findings, field edits and full parent acceptance remain open.

## Candidate 34758 focused Bottom/blocked-finding observation

- Pinned TypeScript source `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/MechanicalAssemblyPanel.tsx:525-562`: selected structural layers expose the Assembly settings return, source-mapped title and dimension fields, resolved body thickness when present, all current error findings in the default-open Fit issues section, and Edit shared closure hardware for Bottom/Retainer with an internal gasket. The shared Case viewer keeps Display controls available beside the Inspector context.
- Dioxus candidate `37d81320712b16cd6901f8e8ea3c80a0833ce576` at `http://127.0.0.1:34758/boardstudio/`, provenance `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f`. A fresh profile imported the same original archive and configured the Case stack. Changing the first closure Boss X from `129.0497589111328` to `128` saved and blocked the preview with the current mount-clearance error. Selecting Bottom showed Bottom thickness, Wall thickness and Clearance; the prior-geometry status remained explicit, current Fit issues exposed that error and its Show action, and Display controls remained visible. `Assembly settings` returned to the global stack editor.
- Candidate screenshot: `dioxus-bottom-blocked-34758.png`, SHA-256 `278b53e2590348e18249e637115a34edcde56f01206d019dfd928737eede3062`.

This is a focused candidate observation for Bottom with a blocked current result. The existing private leaf already implements the other source-mapped fields and Retainer/Bottom shared-hardware return predicate. It does not establish a paired Bottom mutation, the remaining layer journeys, or Issue12/parent acceptance. A separate source caveat remains: the root `on_show_mechanical_finding` route looks up findings from an exact current `CadScene`, while this blocked finding is projected from `MechanicalResolution`; the Show button's navigation result is therefore not claimed here.

## Blocked finding-navigation RED and source repair

- React `5a472a9426e6e38993361da402cd4ec730feb369`, fresh profile `case-show-react`: on the same original5b archive, configure Case and set the first closure Boss X to `128`. The blocked diagnostic was labeled `Generated · plate`; clicking its `Select affected geometry` action selected the Plate object-tree row and opened the Plate contextual Inspector.
- Dioxus candidate `37d81320712b16cd6901f8e8ea3c80a0833ce576`, fresh profile `/tmp/agent-browser-case-issue12-34758`: repeat X=`128`, return to Assembly settings, then click `Show` on the current blocked mechanical finding. The tree remained on `Left case assembly`; no generated layer was selected. This is the expected RED from the old route requiring an exact current `CadScene` and looking only for direct layer IDs.
- The isolated source repair carries finding navigation with exact captured scope/token/revision and the matching current `Rc<MechanicalResolution>`. Root admission retains its current session/owner guards; layer mapping now also finds the generated layer whose body owns a targeted mount ID, matching React's `findingTarget` behavior. Candidate GREEN and integrated affected check remain pending.
