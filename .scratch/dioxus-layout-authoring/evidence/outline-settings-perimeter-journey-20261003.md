# F3.4 Outline Inspector paired journey — 2026-10-03

This receipt records a manual paired comparison on the pinned React workbench at `http://127.0.0.1:5173/` and Dioxus candidate at `http://127.0.0.1:34742/` (integrated source `19525578207ff66bfb51a6c7a6483a0bb7905029`). React oracle is pinned source `5a472a9426e6e38993361da402cd4ec730feb369`. Both browser sessions used QA copies of Sofle v2; no original fixture was overwritten.

## Observed interactions

- Both copies contain a `QA Outline` fixed version created from Generated and renamed through the existing Inspector workflow. The Dioxus tree showed Generated and QA Outline; the React tree showed the same two rows.
- Dioxus applied a Chamfer size edit and showed the updated accepted value before reload. An Undo followed by Redo restored the changed corner style/value in the Inspector. This was a one-sided history probe, not a paired history acceptance journey.
- React exposes a real perimeter point editor. The QA copy entered the editor, selected point 2, inserted a point (34 → 35), then Undid the operation (35 → 34). The original saved perimeter remains unchanged.
- Dioxus 34742 has no perimeter editor; its Board outline Inspector exposes version/settings actions only. The Dioxus point editor is in isolated candidate `a4f51c57a67f55e0fb59c1e5150547cd73869dbf` and was verified with mounted production-hook WASM tests, but was not rebuilt into this public page.
- On reloading each QA copy, React reopened with QA Outline active. Dioxus retained the copied/renamed QA Outline row, but selecting that row showed Active outline=Generated and Chamfer size 2. The displayed Active outline value did not match the active QA Outline; later boundary inspection below distinguishes this control mismatch from persisted data loss.

## Retained captures

All captures are in `/home/chris/.local/share/boardstudio/retained-tmp/20261003/layout-outline-journey/`:

- `react-perimeter-editor.png` — React point editor before insertion; 34 accepted points.
- `react-final-perimeter.png` — React editor after the local insert/undo cycle.
- `dioxus-final-settings.png` — Dioxus fixed-version settings before the later history probe.
- `dioxus-reopened-fixed-settings.png` — Dioxus after reselecting QA Outline before the second reload.
- `dioxus-reopen-generated-after-select.png` — Dioxus after reload and selecting the QA Outline tree row; the Active outline combo still reads Generated.
- `react-reopened-outline.png` — React after reload and selecting the QA Outline row; Active outline reads QA Outline.
- `dioxus-3d-outline.png` and `react-3d-outline.png` — paired 3D Inspector/context view. React exposes Shaded/Wireframe/Hybrid, hidden-line, Fit, Top, Bottom and Isometric controls; Dioxus exposes Fit case and Assembly layers. This remains a broader F3.6/F7.3b parity gap.

## Gate status

The source-level F3.4c mount tests prove the bounded Generated `CopyOutline.edit` and fixed `SetOutline` routes through the production hook, including coordinate Enter/Escape behavior. They do not prove the public Dioxus point-editor journey. Copy and rename succeeded in both QA copies; the save/reopen comparison exposed the Active outline control mismatch above, so that public control parity remains open pending the repaired candidate. Settings fidelity, two-sided Undo/Redo, contextual-pane and responsive acceptance also remain open. F3.4 and F3.7 parent criteria remain open; this receipt does not close them.

## Reopen diagnosis correction — 2026-10-03

Read-only inspection of the same retained Dioxus session confirms the persisted `boardOutlines` row has activeVersionId `outline-version-18`, version name `QA Outline`, and Chamfer size **2** at revision12. The tree labels QA Outline Active; the Inspector describes a fixed outline and its version-name field reads QA Outline. These are projections of the accepted fixed version. Size2 matches the persisted record, so this evidence does **not** establish lost corner settings or a load/persistence reset. The mismatch is the native Active outline select: its value is empty/Generated after dynamic version options mount, despite correct accepted data. The repair explicitly marks the option matching the accepted activeVersionId as selected. Original PNGs and the original receipt are retained; no original browser state was rewritten.

The original receipt is preserved at `/home/chris/.local/share/boardstudio/reviews/outline-settings-perimeter-journey-original-20261003.md`. The precise saved-data and DOM observations are in `outline-reopen-select-fix-20261003/retained-browser-red.json`. Public corrected-candidate reopen remains a focused follow-up; perimeter editing and full F3.4/F3.7 joins remain open.
