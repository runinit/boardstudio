# Compact tree handoff — independent Spec patch review

Approve /tmp/frontend-run/compact-tree-handoff.patch as the bounded presentation correction. Compared its exact diff with pinned React Workbench.beforeNavigate, useWorkbenchTree and useWorkbenchSelection. No source edits/compiler/browser execution.

Board activation keeps panel state unchanged, matching the board branch’s direct context/selection handling. LayoutGroup closes Objects without forcing Inspector, matching its Design navigation branch. Matrix/Row/Column/Key/Component close Objects and open Inspector, matching navigation plus selectScope. Existing disclosure-only controls are unaffected.

The patch retains submit_context as the selection authority. It checks captured generation, full Scope and fresh context validity before submission, then rechecks generation/Scope before changing panel state. A stale callback cannot close/open panels in a new scope. There is no new selected-ID store, document edit, history operation, camera action or public API.

This approval covers source logic only. Preserve the actual paired public red and require rebuilt green for pointer/keyboard selection, empty context, Board/LayoutGroup exceptions, disclosure independence, focus/compact behavior, desktop modes and revision/history neutrality. It does not complete T1-10 or change later drawer thresholds. No new RF.
