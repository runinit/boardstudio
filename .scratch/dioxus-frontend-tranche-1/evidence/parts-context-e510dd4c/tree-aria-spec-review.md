# Objects ARIA correction — independent Spec review

Decision: approve f65b0c83 against parent e510dd4c for the bounded semantic ownership correction. No material source finding. Reviewed the full 5-insertion/4-deletion objects.rs change, surrounding handlers/scope validation/CSS, the diagnosis and paired raw axe reports. No compiler, browser or source edits.

The existing row now owns treeitem role, level, selected and conditional expanded state. Its aria-labelledby references the unchanged selection-button ID, retaining that control’s label/detail while excluding the separate disclosure label. The tree therefore owns row treeitems, each containing its existing controls; the disclosure is no longer an unowned sibling of a treeitem under the tree. Existing IDs, hierarchy order, indentation and selection CSS remain unchanged.

Selection and disclosure remain distinct native buttons. No row activation handler or tab stop is added. Pointer callbacks, Enter/Space prevention and dispatch, arrow-nudge eligibility, Shift increments, board-expansion behavior and disclosure-only toggles are byte-for-byte unchanged. The attribute move introduces no second selection submission, document edit, navigation action or public API. Current-context scope validation and captured full-Scope selection/nudge requests are unchanged; invalid context cannot acquire selected metadata through this change.

Both retained axe reports independently contain the critical aria-required-children finding: disallowed button[aria-label] children under the candidate .m1-component-list and React .wb-tree-viewport. The fix addresses that exact ownership cause. Reference failure is not an accessibility waiver.

Source approval does not certify runtime accessibility. Require rebuilt same-fixture axe to clear the original violation without new nested-interactive/name/role failures; verify accessible names and row selected/expanded/level state, Tab focus, independent disclosure and selection by Enter/Space, pointer selection, real-part arrow nudge/Undo, and disclosure/selection history neutrality. Actual assistive-technology announcement with focus on descendant controls remains an explicit unpassed gate where unavailable. Retain broader T1-10 scope, compact/theme and shared acceptance joins. This repair does not adopt APG arrow navigation or expand Outline/Bridge scope. No new RF.

Reviewed objects.rs SHA-256: e0175325eebebfad9f14c429fd7f5a845f9c83f786c0b5dcbd8225fcdbd67c0b.
