# F3.8 — Geometry script editor

**Parent:** F3.8 Geometry script editor in `.scratch/dioxus-frontend-v1/issues/03-layout.md`; this child does not accept F3.5 or F3.7.
**Reference:** React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/useScriptEditor.tsx`, `app/src/ui/Workbench.tsx`, and the existing script e2e scenario in `app/e2e/workbench.spec.ts`.
**Existing owner:** `ProjectDoc.scripts`, Core `script::apply_scripts`, `ReplaceDocument` edit history, and scene findings. Rhai evaluation and generated geometry remain entirely in Core.

## User workflow

From Add object → Board geometry → Geometry scripts…, the Inspector opens the Geometry scripts page. Choosing it from PCB switches back to Layout. The page has a Back to selection action, a Scripts heading, and + New script. A newly added record is accepted as `Script N`, selected, empty, and disabled. When scripts already exist, Active script selects one; changing selection loads its accepted Name, Rhai source, and enabled state into the editor. These are local drafts until Apply.

Apply is disabled for empty or whitespace-only Rhai source. For nonblank source, Apply commits the current name, source, and Enable on Apply checkbox together as one ordinary `ReplaceDocument` edit. Core evaluates enabled scripts while handling that edit. Accepted generated parts/outlines and current Core findings come from the accepted scene. A rejected Core apply leaves the prior accepted document and scene in place and keeps the user's draft available to correct. There is no live evaluation while typing.

The page can be closed with Back to selection or Escape. The ordinary Editor Undo/Redo controls remain the history authority. New script and Apply each use one edit/history entry; saving and reopening preserves script records and regenerates the same enabled outputs through the existing Core open path.

## Boundaries

- The Dioxus module owns only selected-script identity and transient Name/source/enabled drafts. `ProjectDoc.scripts` remains the only durable script state.
- The root Editor owns workspace switching, Inspector visibility and page routing. The private editor submits against the captured accepted snapshot only while the same session/token/revision remains Ready.
- Core owns script validation, Rhai evaluation, stable generated IDs, generated part/outline projection, findings, rejection and history. The UI adds no language, execution, geometry or persistence policy.
- Script finding navigation remains the F3.5 shared findings responsibility. This editor presents current Core findings and the apply failure feedback but does not create a parallel target-routing system.
- Existing browser/Core bridge boundary is unchanged: the UI submits one typed `Event::Edit` with `EditOperation::ReplaceDocument`; existing Core/worker machinery carries and executes it. Retirement condition for the React bridge is the integrated paired F3.8 route/draft/apply/error/history/reopen journey plus its parent acceptance; the migration coordinator owns removal with equivalent behavioral coverage.

## Acceptance evidence

Use the pinned layered Sofle `.boardstudio` fixture (`5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) and record exact React/candidate source, provider provenance, route, browser errors, and screenshots. The changed journey covers Add object routing, new disabled script, name/source/enabled draft and Apply, accepted generated result/findings, and Undo/Redo or save/reopen as appropriate to the current candidate. Reuse the existing React `runs a bounded script to add an outline hole` characterization. Do not add a broad matrix of neighboring UI tests.

No new refactoring takeaway is recorded for this child: the existing script schema and Core engine already own durable state and behavior; only the missing Dioxus presentation adapter is added. Preserve RF-001, RF-006, RF-009 and all existing Layout parent criteria.
