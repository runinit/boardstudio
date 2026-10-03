# F3.8 Geometry scripts editor

## Pinned reference and candidate gap

- React source: `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/` in named session `f38-geometry-scripts-react-5173`.
- Dioxus RED source: `900068a0df413059732f9f365abd598734b7f86c`, served at `http://127.0.0.1:34762/boardstudio/`, provider provenance SHA-256 `319f378ba0d2eb0855059d2368e1e8028c0c6267e0a50a22b9ed8e8a195e1251` in named session `f38-geometry-scripts-red-34762`.
- React browser source path: `app/src/ui/useScriptEditor.tsx`; the existing bounded Core-backed e2e characterization is `app/e2e/workbench.spec.ts` (`runs a bounded script to add an outline hole`).
- Fixture: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- The browser sessions `f38-geometry-scripts-react-5173`, `f38-geometry-scripts-red-34762`, and `f38-geometry-scripts-red-full-34762` are closed and had empty browser error buffers.

On React, Add object → Board geometry contains `Board outline…` and `Geometry scripts…`. Opening Geometry scripts shows Back to selection, Scripts, and + New script. After creating a new script, `Script 1` is selected, `Name` is `Script 1`, `Rhai source` is empty, `Enable on Apply` is unchecked, and Apply script is disabled. The selector, three draft fields, Core findings list, and Apply action match `useScriptEditor.tsx`. This is the current pinned behavior, not a new script language contract.

On the Dioxus RED candidate, the imported fixture shows Board outline… but no Geometry scripts… entry in the Board geometry menu. Evidence:

- [React Geometry scripts with a new empty script](react-geometry-scripts-new-empty.png)
- [Dioxus Add object missing Geometry scripts](dioxus-add-menu-missing-scripts-red.png)

## Ownership and scope

`ProjectDoc.scripts` and Core `script::apply_scripts` already own persistence, Rhai execution, generated geometry, findings and history. The Dioxus child adds the contextual route, transient form drafts and calls the existing `ReplaceDocument` edit path against the current accepted snapshot. No second store, script runtime, language feature, live-run or delete affordance is introduced. Invalid enabled source is rejected by Core; the accepted document/scene remain authoritative and the draft remains editable.

The root Editor owns workspace switching and the Inspector-page callback. The private editor only commits while its captured accepted session/token/revision is still current and Ready. The existing Core/worker boundary remains unchanged: one typed edit command reaches the existing Core owner. React removal remains gated on integrated F3.8 route, draft/reset, Core output/findings, rejection, history and reopen parity with equivalent behavioral coverage. RF-001, RF-006 and RF-009 are preserved; no new refactoring takeaway was observed.

## Candidate GREEN receipt

Pending: record the exact integrated candidate source/provider and one actual script add/edit/apply journey with accepted generated result and Undo/Redo or save/reopen. The assigned combined affected page check and consolidated review remain with root. No broad UI matrix is required.
