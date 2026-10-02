disposition: fix
Missing inputs: no mobile capture or saved five-block direction contract / quality-bar card was supplied; review is limited to the named desktop captures, existing PRODUCT.md, and the listed React/Dioxus files. Substitution: no browser; reviewed the supplied captures and source only.

persistence
Pass for this scoped review: PRODUCT.md and DESIGN.md exist in the supplied worktree; the change is code-led and no comp-round state is expected. The React reference and current Dioxus capture identify the compared surfaces. No mobile capture, hook findings, or explicit direction/quality-bar packet was supplied.

fidelity
| Element | Verdict | Evidence |
|---|---|---|
| Layout canvas and board contour | match | Both screenshots show the same dark canvas and green keyboard-board outline. |
| Placed-part courtyard/reference marks | acceptable adaptation | Current view has placed-part outlines and references, though the crowded references dominate where the reference separates key outlines from board details. |
| TYPE | acceptable adaptation | Both use compact, restrained sans-serif UI labels; exact font evidence is limited by the captures. |
| MATERIAL | match | Both present CAD geometry as clean vector outlines; no physical material is imitated. |
| GROUND | match | Both captures use a cool, dark blue-black canvas field. |
| Keys visibility switch | missing | Reference Layers panel has a working Keys switch; current capture reports no layer control. |
| Components visibility switch | missing | Present in React's layer panel; absent in current Dioxus capture. |
| Keycaps visibility switch and default outline | missing | Reference has the Keycaps layer enabled and shows keycap outlines; current capture reports zero keycap-outline nodes. |
| Footprints visibility/view | missing | React has a Footprints layer switch and a separate Footprints canvas view option; current capture reports no footprints control and shows no pad/footprint details. |
| Board visibility switch | missing | Reference exposes Board in the same layer panel; current capture has no layer controls. |
| 2D / 3D assembly / Footprints view selector | missing | The reference's upper-right selector is absent from the Dioxus canvas. |

ceiling
Unused native devices: the compact floating Layers panel with per-layer swatches and eye state, plus the canvas's 2D / 3D assembly / Footprints view selector. Both are directly relevant controls in the incumbent reference and should remain findable without displacing board-editing space.

material_fixes
1. Add the reference's Layers panel with functional Keys, Components, Keycaps, Footprints, and Board switches; each control must change only its corresponding canvas geometry and expose its current state accessibly (fidelity: missing layer functions).
2. Draw the resolved keycap outline for each applicable key by default, and connect the Keycaps switch to that overlay (fidelity: missing keycap layer; current keycapOutlineCount is 0).
3. Render the actual footprint details when Footprints is enabled and connect the Footprints switch/view to that state; keep the reference's default layer state, where Footprints are hidden (fidelity: missing footprint view/details).
4. Restore the 2D / 3D assembly / Footprints view selector as actual view navigation, rather than a decorative control (fidelity: missing reference affordance).
5. Keep Board visibility independently toggleable while preserving existing part editing, selection, undo/redo, theme, and compact layout behavior (fidelity: missing Board switch; contract scope).

keep
Keep the incumbent cool dark CAD canvas, green board contour, compact editor framing, and existing Dioxus part editing/selection/history/theme behavior while adding the missing canvas layers and view state.
