## verdict
1. Resolved — Compact navigation remains usable at 390px; the open project menu stays within the viewport (`compact-menu.json`).
2. Resolved — In both compact panel states, the footer ends where the saved-status strip begins, and hit-testing reaches Undo (`expanded-panels-footer.json`, `closed-panels-footer.json`). A compact numeric edit followed by Undo restored the original value (`undo-result.json`). The recaptures show Undo/Redo visible in the closed-panel and expanded-panel views.
3. Resolved — The normal saved-state toolbar omits the disabled Retry save action in desktop captures.
4. Resolved — Object name and type remain visually separated and have a coherent accessible label in the compact DOM proof.
5. Resolved — The supplied axe runs report zero violations; the heading and single main landmark remain present.
6. Resolved — Theme control typography remains on the documented 12px role.

## remaining
Clear for the six scored fixes. Full F2 panel resizing/drawers and F3 canvas parity remain deferred; axe contrast evaluation for SVG canvas labels remains incomplete and is carried with F3. This verdict covers only the six reviewed fixes.

disposition: ship
