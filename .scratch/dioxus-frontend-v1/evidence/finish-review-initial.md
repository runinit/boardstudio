disposition: fix
Missing inputs: no Impeccable QUALITY BAR card or formal direction contract/seed key was supplied; the supplied F1 ticket, v1 roadmap, PRODUCT.md/DESIGN.md, detector findings, DOM evidence, and captures were used. Reference images were treated as parity references.

## persistence
Pass. PRODUCT.md and DESIGN.md exist. This is a code-led F1 port; comp build state is not applicable. The F1 tracker remains implementing, consistent with the visible first increment. Detector findings were supplied and not rerun.

## fidelity
| Element | Verdict | Evidence |
|---|---|---|
| Desktop workbench topology and workflow labels | acceptable adaptation | Desktop keeps Objects / canvas / Inspect, and the exact Layout, PCB, Keymap, Keycaps, Case, Parts order plus Export. It retains M1 geometry/editor behavior while F1 replaces the shell. |
| Compact workspace navigation | contradicted | At 390px, the selector is only 12px wide (`/tmp/frontend-compact-red.json`), and opening the project menu expands the page to 405px. The selector is not practically reachable; the compact reference retains a legible workspace choice. |
| Compact panel access and reading order | contradicted | Current captures stack the canvas, Objects, and Inspect into a long page; the panel/navigation affordances are unclear once the topbar is compressed. Preserve reachable panel controls while compacting the shell. |
| Project/save controls | contradicted | A disabled “Retry save” is always visible in the canvas toolbar during a normal saved state. The retry action belongs with a failed save state; the idle state should not present a dead primary control. |
| Objects row names | contradicted | Labels run together (for example `main-right-keys-SW1Switch`), making the object type difficult to parse. Separate the reference and type visually and in the accessible name. |
| Heading and landmarks | contradicted | DOM evidence reports zero headings, an `aria-label` on a generic brand span, and nested/duplicate `main` landmarks (`/tmp/frontend-review-red.json`, `/tmp/frontend-axe-red.json`). Give the shell a real heading/brand name and keep one main landmark. |
| Light and dark shell tokens | match | Both themes render with consistent panel/canvas roles and readable geometry separation in supplied desktop and compact captures. |
| System theme and persistence | acceptable adaptation | Source implements browser preference observation and local storage; no supplied capture exercises a live System preference change. |
| Canvas geometry/labels versus reference | deferred to F3 | The reference canvas is cleaner and its object tree is grouped; the task explicitly carries M1 canvas behavior and defers full canvas parity to F3. Do not spend F1 fixes on geometry redraw. |

## ceiling
Cannot score against the missing QUALITY BAR card. In the supplied captures, the unused device with immediate F1 value is the reference’s compact, explicit panel/workspace navigation; current compression hides that affordance. No material, imagery, or motion treatment is part of this operate shell’s F1 promise.

## material_fixes
1. Check 4 / compact-navigation promise: give the compact workspace selector usable width and prevent the open project menu from increasing document width beyond the viewport; retain a clear panel/workspace path at 390px.
2. Check 4 / compact-panel promise: make Objects and Inspect reachable through clear compact controls, using simple collapsible panels if needed. Full resizing and drawer behavior stays in F2; F1 needs legible, reachable access and a deliberate reading order.
3. Check 6 / floor states and F1 save behavior: show Retry save only when save durability has failed, with the associated failure status; remove the always-visible disabled action in the normal saved state.
4. Check 4 / Objects promise: render component reference and type as distinct readable fields (and one coherent accessible label), such as `main-right-keys-SW1 · Switch`.
5. Check 6 / semantic coverage: add a real level-one app/workspace heading and use the brand as a properly named element; replace the nested `main` with a non-main container so the document has one main landmark.
6. Detector advisory / typography: review the 11px literal sizes in `web/assets/m1.css` against DESIGN.md’s 10/12px roles, particularly the compact Theme control. Preserve the compact density if a deliberate role justifies the intermediate size; this advisory alone does not require redesigning the type ramp.

## keep
Keep the restrained BoardStudio palette, the recognizable three-pane desktop workbench, the exact six workflow labels plus Export, and the existing M1 canvas/session behavior while F1 shell defects are corrected.
