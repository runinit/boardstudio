# Case workspace shell parity

## Problem statement

The Case workspace still presents generic board and physical-instance selectors, a project heading, text visibility controls, and status copy that competes with the 3D canvas. React presents assembly roots and their contextual children in Objects, uses eye controls for visibility, and keeps the Case viewer and its common view controls in the work area.

## Solution

Make the Case work area canvas-first and keep assembly navigation in the Objects tree. Keep the existing accepted board/instance scope, Case display preferences, selection routes, mechanical settings, body editor, preview lifecycle, and renderer as the authorities. Restore the Case-only inline render-mode and camera controls while leaving specialist view settings available in the existing disclosure.

## User stories

1. As a Case designer, I want to navigate between real board or physical-assembly roots in Objects, so that the tree describes the assembly I am editing.
2. As a Case designer, I want to expand the active assembly to see its current generated or authored rows and PCB components, so that I can select the relevant object in context.
3. As a Case designer, I want to show and hide represented assemblies, layers, and components with eye controls, so that visibility changes remain easy to scan and preserve their existing display-preference behavior.
4. As a Case designer, I want the 3D view to use the available canvas height and expose common render and camera controls inline, so that I can inspect the assembly without opening nested menus.
5. As a Case designer, I want readiness and delivery errors to remain truthful without displacing the canvas with duplicate implementation-status copy.
6. As a Case designer, I want the selected-part breadcrumb, canvas badge, and existing Inspector/editor to remain scoped to the current accepted board and instance.

## Implementation decisions

- Refine the existing Case Objects and Inspector composition; do not create a second board/instance selector or selection store.
- Put board and physical-instance navigation on the real assembly-root rows. Keep current scope/token/generation guards on navigation and display changes.
- Use icon-only visibility buttons with accessible Show/Hide labels and the existing Case display-preference callbacks.
- Enable inline camera and render-mode controls only for Case viewers; Layout, Keymap, Keycaps, and Parts viewer consumers keep their current composition.
- Keep specialist assembly, section-plane, hidden-line, and layer appearance settings in their existing disclosures.
- Keep preview status and model-delivery failures actionable and truthful; remove redundant ready-state paragraphs from the canvas flow.

## Testing decisions

- Reuse the existing paired original-Sofle archive and current Case selection, model-delivery, history, and viewport receipts for unchanged behavior.
- On the next integrated candidate, compare the settled React and Dioxus Case shell at the same desktop viewport and verify assembly-root navigation, eye visibility, inline display mode, and canvas containment. Do not add routine UI tests or repeat unchanged edit/history matrices.
- Run the combined affected strict page check through the coordinator's assigned build slot.

## Out of scope

- New renderer, engine, project-format, geometry, or selection authority.
- Replacing the existing responsive panel navigation, mechanical editor, generated/authored Case workflows, or specialized viewer settings.
- Closing F7.2, F7.3, F7.4, INT.2, BND.1, or F7.8 acceptance.

## Further notes

This is a follow-on to Issue 09. It preserves the selected-part summary and visibility preference owners, and changes only the Case shell composition and its Case-only viewer controls. Existing RF-001/RF-009 accounting remains applicable; no new refactoring finding is asserted by this packet.
