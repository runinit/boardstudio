## verdict
1. Resolved — Compact navigation is 100px wide at 390px, and the open project menu leaves page width at 390px (`compact-menu.json`).
2. Partial — Objects and Inspect now have reachable controls; the expanded panel capture shows visible position inputs and a coherent object label. The compact footer remains obscured: the footer occupies y=781.6–815.6 while “Saved locally” covers that same bottom band in the capture, leaving Undo/Redo unavailable at a glance (`compact-footer-probe.json`, `compact-dark.png`).
3. Resolved — The normal saved-state toolbar contains the project name without the disabled Retry save action in desktop light/dark captures.
4. Resolved — Object name and type are separated in the captures; the compact DOM proof reports one coherent accessible name, `main-right-keys-SW1, Switch`.
5. Resolved — The supplied desktop and compact axe runs report zero violations; the heading and single main landmark are present per the supplied final-source evidence.
6. Resolved — The former 11px literals were brought to 12px roles, including the compact Theme selector, consistent with the documented type roles.

## remaining
Restore a visible compact Undo/Redo path without overlapping the saved-status strip; the compact editor’s history controls are still obscured. Full F2 panel resizing/drawers and F3 canvas parity remain deferred. Axe’s contrast scan remains incomplete for the overlapping SVG canvas labels, carried with the F3 canvas work.

disposition: fix
