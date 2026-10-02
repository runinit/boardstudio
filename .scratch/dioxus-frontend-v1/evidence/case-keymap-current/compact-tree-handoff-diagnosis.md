# Compact tree selection handoff — proposed private patch

**Diagnosis:** valid tree selection reaches Session, but `Editor::select_tree` never closes Objects or opens Inspect. The existing Parts activation callback already changes those two presentation signals. The proposed patch adds the reference tree behavior to the tree callback without changing selection helpers, Session, public APIs, CSS, configuration, or persisted preferences.

Patch: `/tmp/frontend-run/compact-tree-handoff.patch` (not applied).

Inspected checkout: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, HEAD `0d299e2c03c8b4df6dc7436f49b71475d5e511ff`.

Base `web/src/presentation.rs` SHA-256: `f10eb06afe74605a7820f6650dfdeafd73423a00fa92f86b8b5b1c3a9dad6657`. Root is concurrently preparing its Keymap mount; preserve that work and recheck the select_tree hunk before application.

## Existing red evidence reused

Packet: `.scratch/dioxus-frontend-tranche-1/evidence/tree-and-parts-4b05d451/ui-verifier/README.md`, `compact-select-{candidate,reference}.json`, `compact-hittest-{candidate,reference}.json`, and `compact-tree-{candidate,reference}-selected.png`.

The supplied public run used the same Sofle archive at 720×640. Both final hit tests targeted actual Key 1.2 selection buttons. Candidate JSON confirms selected Key 1.2/left-keys-SW7 and one-key context. The two screenshots visibly show candidate Objects remaining open with no Inspector and reference Inspect showing Key 1.2. The verifier records candidate Inspect collapsed/Selected context 0×0. Earlier offscreen clicks are excluded from this diagnosis. This task re-read the records and viewed both screenshots; it did not rerun the browser or claim a fresh failing test. Reuse of this precise supplied red evidence replaces reproducing the already-confirmed failure during the patch-only task.

Ranked hypotheses were missing callback panel-state updates, rejected selection, or CSS hiding an opened panel. Successful candidate selection/context falsifies rejection for this trace. `select_tree` (`presentation.rs:652`) only calls `selection::submit_context`; that helper updates semantic context/selection/anchor but has no panel signal access. `panels.rs:98–100,174–184` and `m1.css:301` hide the Inspector when its compact signal remains false. Thus CSS reflects the unchanged state; no CSS correction is needed.

## Exact reference conditions

`WorkbenchTree.tsx:53` invokes each entry's onSelect. This tree path uses `selectScope`, not choosePart. For Layout selections, `useWorkbenchTree.ts:265–267` wraps entries with `changeMode('Design')`. `useWorkbenchNavigation.ts:33–35` invokes beforeNavigate even when already in Design; `Workbench.tsx:106` closes Objects. `useWorkbenchSelection.ts:87–112` opens Inspect on every selectScope activation, even with zero live selected parts.

| Current Dioxus context | Objects | Inspect | Reference source |
| --- | --- | --- | --- |
| Matrix, Row, Column, Key, Component | Close | Open | Wrapped entries plus selectScope; layout-name entries use the same matrix path at `useWorkbenchTree.ts:320` |
| LayoutGroup (half group) | Close | Preserve prior state | `useWorkbenchTree.ts:295`: changeMode, setScope, setSelected; no selectScope/setRightOpen |
| Board | Preserve prior state | Preserve prior state | `useWorkbenchTree.ts:132`: clears scope/selection and may expand; no panel actions |
| Disclosure / context-free grouping | Unchanged | Unchanged | Separate disclosure handling; does not enter select_tree |

Empty keys/empty groups using the five inspector-bearing contexts must still open Inspect. Do not gate handoff on a nonempty selected-ID list. Repeated activation of an already-selected item must also hand off; no selection-change effect is used.

## Proposed change and checks

The patch keeps the existing `submit_context` call. Before calling it, the root callback checks captured generation, full Scope, and current semantic context so a rejected stale/invalid request cannot move panels. After submission it rechecks Scope/generation before applying the context-specific panel changes. No document event is added beyond the existing SelectParts path; no settings write or helper-contract change is made.

Executed: `git apply --check /tmp/frontend-run/compact-tree-handoff.patch` — passed against the inspected source. This validates patch applicability only. No repository source edits, compilation, formatting invocation, live browser replay, or passing regression assertion were performed here.

Root must obtain independent review and verify the integrated change with a fresh public candidate. Required focused replay: same 720×640 Sofle Key 1.2 selection, Objects closed/Inspect open/visible selected summary, then reopen Objects and reactivate the same key; pointer and Enter/Space; Matrix/Row/Column/Component and empty-key contexts; Board and half-group exceptions; stale scope/generation no panel movement; desktop selection unchanged and revision/history/storage neutral. Retain the red packet alongside the fresh green result and source provenance. This patch proposal does not close T1-09/T1-10 or broader acceptance.
