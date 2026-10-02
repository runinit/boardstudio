# Independent Standards: tree row semantics and dead helper removal

Reviewed exact `e510dd4c...f65b0c83` in `frontend-tree-20261002`: one commit, `objects.rs` only, five additions/four deletions. Also checked `cfa64662...e510dd4c` catalogue helper deletion. No source mutation, compiler or browser execution. Tree diff whitespace check passed.

**No new material Standards findings.**

The existing row now owns `treeitem`, level, selection and conditional expansion state; its two controls remain native sibling buttons inside that row. `aria-labelledby` points to the existing selection-button ID, retaining its label/detail without incorporating the disclosure's action name. The selection button no longer duplicates the treeitem role or tree-only metadata. No handler, focusability, ID, style, mutable authority, ownership lifetime or API changed.

This places the row directly under the tree and the disclosure under the row, addressing the specific unowned-button shape in both recorded critical `aria-required-children` reports (`/var/tmp/tree-515-a11y.json`, `/var/tmp/tree-react-a11y.json`). It is consistent with [WAI-ARIA 1.2 treeitem context and naming](https://www.w3.org/TR/wai-aria-1.2/#treeitem). The exact [axe 4.12.1 role metadata](https://raw.githubusercontent.com/dequelabs/axe-core/v4.12.1/lib/standards/aria-roles.js) does not mark treeitem children presentational; its [nested-interactive matcher](https://raw.githubusercontent.com/dequelabs/axe-core/v4.12.1/lib/rules/nested-interactive-matches.js) uses that property. Source inspection therefore supports this bounded semantic repair without treating it as verified public green.

Actual announcement of selected/expanded row state while a descendant button has focus remains an AT gate. Rebuilt axe, accessible names/levels, Tab/Enter/Space, independent disclosure, selection/nudge/Undo and focus checks remain necessary. Existing flat-level projection and reference arrow-nudge policy are unchanged; this does not certify full APG keyboard conformance.

The deleted `catalogue.rs::category_label` had no callers at `cfa64662`; actual grouping labels come from `group_choices`. The similarly named `details.rs::category_label` is separate and retained. Removing the unused private function is behavior-neutral by source inspection and reduces one duplicate mapping. Strict-check results reported by root were not rerun here.

RF: no new architectural takeaway. This local semantic ownership correction preserves the existing presentation boundary. Whole T1-10/11 acceptance and actual assistive-technology evidence remain open.
