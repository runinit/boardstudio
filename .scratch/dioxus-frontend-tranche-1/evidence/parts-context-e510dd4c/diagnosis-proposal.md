# Private Objects tree accessibility diagnosis and proposal

Status: read-only diagnosis/proposal. No source or DOM edits, compiler/build, new browser run, or claimed green. Candidate evidence is source515f390d at34655. Do not use this as independent approval of b51353bb, which this agent authored.

## Exact red and cause

Recorded axe4.12.1 `/var/tmp/tree-515-a11y.json` and `/var/tmp/tree-react-a11y.json` both contain critical `aria-required-children`: the CAD structure tree exposes disallowed `button[aria-label]` children. Candidate target is `.m1-component-list`; React target is `.wb-tree-viewport`. Candidate has one violation and separate incomplete color-contrast items. Incomplete results are not passes.

Current `web/src/presentation/objects.rs` creates:

```text
tree "CAD structure"
  generic .m1-tree-row
    button "Expand/Collapse …"  # disclosure
    button role=treeitem       # select/nudge, level/selected/expanded metadata
```

Generic rows do not establish an accessibility ownership boundary, so disclosure buttons become siblings of treeitems under the tree. This matches the exact reported node and explains both frameworks' baseline failure. It is independent of the repaired hierarchy projection/defaults.

Ranked explanations: (1) disclosed buttons are unowned siblings, confirmed by both reports and source; (2) stale/incorrect treeitem roles, contradicted by emitted explicit roles in captured source; (3) an unrelated toolbar inside the tree, contradicted by target markup and loop boundaries. No speculative engine/framework cause is needed.

## Smallest proposed semantic correction

Limit changes to the Objects row markup:

1. Put `role=treeitem`, existing `aria-level`, `aria-selected`, and conditional `aria-expanded` on the existing `.m1-tree-row` div.
2. Name the row with `aria-labelledby` pointing to the existing selection button's stable ID, so the treeitem name remains label/detail rather than including “Expand/Collapse”.
3. Remove treeitem role and tree-only metadata from the inner selection button. Keep it a native named button, retaining its ID, CSS classes, click/Enter/Space/arrow-nudge handlers and normal focusability. Keep disclosure as its separate native named button with its current expansion state and handler.
4. Add no row activation handler or additional tab stop: doing so could double-submit or change the established two-control interaction. Do not hide disclosure controls from accessibility, nest native buttons, remove tree semantics, or suppress the axe rule.

Result: tree → row treeitem → two native controls. No visual CSS/layout changes, domain edits, selection-adapter changes, API changes or Outline/Bridge scope expansion. WAI-ARIA1.2 requires treeitem ownership under tree/group and permits named treeitems. The axe4.12.1 nested-interactive rule only matches roles with presentational children; its treeitem metadata does not mark children presentational. These support the proposal, but do not replace execution or actual assistive-technology evidence.

This is a bounded ownership repair, not a claim of complete APG tree keyboard conformance. The reference uses arrow keys to nudge eligible parts, so replacing its input policy with conventional tree arrow navigation would be a separate behavior decision.

## Acceptance and authority

Being present in React establishes provenance, not accessibility acceptance. Tranche ACCEPTANCE.md explicitly requires keyboard, focus and axe checks plus actual assistive-technology evidence where applicable; it also says newly discovered reference defects do not automatically authorize behavior changes. The coordinator should record the concrete decision to correct semantic ownership while retaining visible/control behavior before source edits. Do not waive the critical finding or certify T1-10 accessibility from screenshot/reference parity alone.

After authorization: rerun the same public imported fixture and axe4.12.1; require aria-required-children gone and no replacement nested-interactive/name/role violations. Verify treeitem names/levels/selected/expanded values, accessible disclosure and selection buttons, Tab focus, independent Enter/Space disclosure, selection/history neutrality, real-part arrow nudge/Undo and pointer selection. Use accessible names in test assertions; row innerText will include its visible disclosure glyph after the role moves. Check screen-reader announcement of selected/expanded row state when its descendant action receives focus; absent AT access stays an explicit unpassed gate. Retain paired baseline reports and new build/source identity.

RF: no new refactoring takeaway observed; this is a local semantic ownership defect, not a reason to replace the presentation architecture.

Sources:
- https://www.w3.org/TR/wai-aria-1.2/#tree and #treeitem
- https://raw.githubusercontent.com/dequelabs/axe-core/v4.12.1/lib/rules/nested-interactive-matches.js
- https://raw.githubusercontent.com/dequelabs/axe-core/v4.12.1/lib/standards/aria-roles.js
