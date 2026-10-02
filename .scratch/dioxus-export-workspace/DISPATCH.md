# Dispatch contract — F8.1 Export workspace

## Ownership and seam

F8.1 starts after accepted INT.1 and has no acceptance joins. Before implementation, name the page-local Export mount and callback contract for current accepted document/selected-board inputs, row readiness/reasons, existing Embed used models preference, and navigation. Keep feature implementation in a private module. Shared route registration/navigation state in `presentation.rs`, `runtime.rs`, global CSS/build wiring and ledgers remain coordinator-owned; coordinator supplies the minimal callback seam serially. Do not add an API, schema, format, provider facade, or engine behavior.

## Source and behavior

Use canonical F8 workflow/spec and the 62-parent graph; compare React Workbench, exports context, Case workspace and `main.tsx` with the current Dioxus Export view. Show exactly the seven design rows: KiCad board, Draft KiCad board, ZMK firmware, KiCad footprints, SVG board outline, DXF board outline, and authored Case STEP (using the reference Case STEP label when generated Case is not active). Preserve the optional generated mechanical package and portable project copy section/action. Show Embed used models only when its existing preference callback is supplied, with the existing default and exact reference label/description. Add no controls absent from React.

Derive disabled/ready states and reasons from the current selected board, wiring/controller/assignments, definitions, outline, authored Case and current generated preview/generation/assembly inputs. Do not infer generated readiness from document data alone and do not invoke providers from this UI ticket.

Escape, the active Export toggle, and WorkflowReturn restore `lastDesignMode`; it records Design, PCB, Keymap, Keycaps or Case, never Parts/Export. Choosing another workspace navigates to it. Review wiring opens PCB for the selected board, clears semantic and part selection, and reveals Inspector. Review case opens Case for the selected board and reveals Inspector while preserving reference selection behavior. All preserve selected-board identity; Escape does not dismiss the app.

## Graph and handoff

F8.1 `start_after: [INT.1]`; `acceptance_after: []`. F8.2 retains `start_after: [F8.1]`, `acceptance_after: [INT.2, BND.2]`; those remain its acceptance joins. Preserve all other F8 and final F2 dependencies. Fixture-backed F8.1 does not wait for F5/F6/F7 providers or F2.3 completion and does not implement provider delivery. Use shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md`; retain paired reference/public Dioxus, navigation, display, accessibility and independent review evidence. Record “No new refactoring takeaway observed” unless implementation evidence supports an existing RF update.
