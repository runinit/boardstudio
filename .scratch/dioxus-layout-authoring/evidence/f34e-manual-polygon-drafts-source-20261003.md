# F3.4e manual outline draft source receipt

This source-only packet compares pinned React `5a472a9426e6e38993361da402cd4ec730feb369` to the mounted Dioxus Layout outline route at base `54f627b1fa551088f34d900d184269f2f13778a6`.

React `OutlineInspector.tsx` exposes Draw addition and Draw cutout; `useOutlineEditor.tsx` owns draft points, preview, undo/cancel/finish and routes first Generated authorship through `CopyOutline.feature`. For fixed outlines it appends the new feature through the existing document replacement edit. Before this packet Dioxus had the version/settings Inspector and perimeter point editor but no mounted manual polygon draft controls.

The candidate adds Draw addition/Draw cutout, canvas point drafting and rendering, snap/Alt sampling, last-point undo, Finish/Cancel/Escape, and exact Outline owner admission. Generated uses existing `CopyOutline.feature`; fixed versions append with existing `ReplaceDocument`; accepted completion is checked through the current outcome/durable settlement path. The source does not claim Connect points, feature-list selection/removal/editor parity, or full draft capture/lifecycle parity.

**Checks:** `rustfmt --edition 2024 web/src/presentation/outline_lifecycle.rs web/src/presentation.rs` and `git diff --check` passed. No Cargo, browser, or test commands were run per the assigned source-only packet; the combined build and public changed journey remain coordinator-owned.

**RF:** preserve RF-001, RF-006 and RF-009. This comparison identified no new refactoring finding or reason to create parallel state.

**Acceptance remains open:** one public paired draft/cancel/commit/history journey, combined source check, and all parent acceptance joins remain outstanding.
