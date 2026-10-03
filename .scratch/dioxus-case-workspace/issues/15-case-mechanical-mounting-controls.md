# 15: Case mounting locations and hardware-feature controls

**Parent:** F7.4 — Mechanical assembly configuration editor.

**What to build:** In the existing Case mechanical settings panel, let designers add, edit and remove suspension mounts and closure screws, and deliberately adopt current clearance-tested suggested locations using the same saved configuration and scoped operation owner as neighboring settings.

**Blocked by:** INT.1 only, matching F7.4's canonical `start_after`. The existing mount resolver and current Case settings controller are the source; do not add geometry behavior.

**Parent acceptance join:** INT.2 remains required for F7.4. This child does not close F7.4 or establish generated geometry/export acceptance.

## Observable slice

- [ ] Add a “Mounting & hardware” group matching React's mounted placement. Render the Suspension mounts, Closure screws, and Suggested mount locations subsections as disclosures, collapsed by default. Show suspension mounts only when mount style is not Gasket; show closure screws for configured stacks. Preserve source conditions: adding closure screws is unavailable with an internal gasket, while existing closure rows remain editable/removable.
- [ ] For each row, preserve stable ID and expose source mount type, X/Y position, hole diameter and boss diameter controls; also expose boss height for bosses. Add creates a hole at (0,0) with 2.5 mm hole diameter and 5 mm boss diameter/height. Remove targets only the current stable ID.
- [ ] Present only clearance-tested suggested mounts from the exact current, non-previous mechanical scene. For internal gasket, the explicit “Adopt closure positions” action replaces closure positions with those candidates. Otherwise, “Adopt suggested mounts” replaces suspension mounts and “Add suggested closure screws” appends the source-shaped boss rows (hole diameter 2.2 mm; height from the accepted stack dimensions). No action runs when the candidate set is stale, absent, or belongs to another scope.
- [ ] Preserve automatic closure initialization only when `closureMounts` is absent, and preserve explicit empty lists. Deliberate adoption remains distinct from initialization. Any operation stays under the existing current Case owner/readiness/saved-revision guard, serial controller, normal ReplaceDocument history and latest accepted configuration patch. Key the controls subtree by the existing owner identity so a board/instance owner change discards stale numeric drafts while same-owner accepted updates retain them.
- [ ] Paired changed journey covers one representative mount edit and one explicit adoption action against the same saved archive, with accepted document values/history verified. Reuse existing setup, save/reopen and Undo/Redo evidence; do not repeat full setup or broaden into the access-opening editor, fastener specifications/critical fits, or Case viewer manipulation.
- [x] Source finding is limited to React/Dioxus control parity in the Case settings panel. No new RF finding; preserve RF-001/RF-006 and their existing history.

**Ownership:** private `MechanicalSettings` presentation, its accepted-scene projection, and the existing `MechanicalSettingsController` only. No shared viewer, public geometry API, schema or persistence route change.

**Status:** active bounded implementation. Issue 05 remains the parent F7.4c contract and remains open.
