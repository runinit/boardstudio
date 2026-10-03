# Parts04 follow-up review — 1a0128ab

**Standards HOLD; Spec HOLD**, limited to the remaining field-draft synchronization/verification issue below. Frozen source `1a0128ab580dccc178510827e45d0502c67e9a18`, clean worktree `/home/chris/.local/share/boardstudio/worktrees/parts04-authoring-planning-20261002`. Preserve the prior exact629 HOLD report; this is a separate repair review. Governing planning remains cleared `e056a8f8f68d9542a6b94b4534a90a74f7d7bfab`.

## Remaining bounded finding

**[P2, Spec] Preserve unchanged pad-field drafts across an accepted pad-ID rename.** The row key at `web/src/parts_custom_definition.rs:161` is now scope+definition+pad.id. This repairs cross-definition reuse, but still remounts the entire PadFields component when the accepted pad ID changes. A dirty X draft is therefore lost even if accepted X remains unchanged. Pinned React `PartsInspectorPanel.tsx` keys the pad fieldset by index and `InspectorControls.tsx` synchronizes each DraftInput only when that field's accepted value changes. This is the remaining case of the original coupled-reset finding, not a new public feature requirement. Use a scope/definition/row identity matching the reference semantics, with current pad ID captured by the updated handlers, and verify a dirty coordinate survives an accepted ID rename and commits onto the latest accepted document.

**Production verification for the same finding remains incomplete.** The new mounted owner/sibling test changes `controls.definition` to simulate a height/Y refresh, but does not update `controls.snapshot` or Runtime's accepted snapshot/document, and does not blur the retained draft after that refresh. It proves display preservation only. The prior629 request requires accepted sibling refresh→blur to preserve both values. Drive the accepted height/ID change through the existing Session/Core fixture, consistently publish its accepted snapshot through the Runtime seam and mounted snapshot/definition props, blur the dirty width/X control, apply its emitted edit through Session/Core, and assert both the refreshed sibling/ID and draft value remain. This bounded test need not execute full persistence or mount the whole Editor.

The author confirmed these points and will retain 1a unchanged while preparing a separate frozen repair.

## Findings closed at this source

- Owner reset now synchronizes courtyard width/height and retires local error on selected scope/definition changes; pad rows have owner-qualified keys. The mounted equal-value owner switch covers courtyard, pad and error reset. It closes prior owner-leak finding subject to the distinct pad-ID lifecycle issue above.
- Width/height blur now compares the scalar draft to its accepted value, so unchanged empty-courtyard blur emits no edit. Mounted real focus/blur checks capture no Runtime event; prior unchanged-courtyard finding is closed.
- Independent accepted-value effects replace the former coupled tuple reset for courtyard and pad fields. That fixes sibling height/Y updates; pad-ID key remount and accepted-refresh commit evidence still require the above follow-up.
- The Add collision gate now has an actual disposable old-rule mutation run at frozen629. The retained raw log fails exactly with actual3 versus corrected4 for `[1,3]`, and the corrected helper remains unchanged. The previous missing executed-red finding is closed.

## Verification and exact evidence

Read the complete frozen repair diff and new mounted tests, compared with the pinned React row/DraftInput semantics, verified clean status and source-range diff whitespace, and independently hashed/read the retained raw red and handoff. No source changes, subagents or heavy tests were performed.

Exact SHA-256 values:

- `web/src/parts_custom_definition.rs`: `54f64483764553721a3a41f233655d2c9c444dda036fff846c93abe2d3cc07ef`.
- `web/src/parts_definition_name.rs`: `9181fd7181cd7e224d01b1bb5dea45e81cf41c4169ccad25e799c52304ddf32a`.
- Handoff: `7d66c1d5838c21f70910c10a624e97a8ee58b87abcb3c75922110a1cf0019ae6`.
- Add legacy expected-red log: `030bbd2b48de858d6ca4cab81e34da4277b9919b325b2a7160aa8c31a54bf042`; command exited101, one expected assertion failure, actual3/expected4.

The author reports corrected native5/5, all-target WASM cargo check, mounted owner/sibling display test1/1 and mounted unchanged-blur test1/1. **No raw green logs were retained for 1a; these are tool-transcript outcomes plus handoff narrative.** The author confirmed that boundary explicitly. Strict all-target WASM Clippy is not claimed. The handoff now correctly describes the pad-ID integration test as Runtime-event capture followed by a separate actual Session/Core advance, and fixes the earlier planning-report hash typo.

## Integration/public boundary

Private edit admission, scoped net remap/removal and one normal ReplaceDocument history route remain as previously reviewed. Required root CSS/shared composition, joined strict Clippy, paired all-fields/error recovery, compact/theme, Undo/Redo, archive and actual save/reopen remain open. This source HOLD is solely the bounded remaining draft-sync repair and its consistent accepted-refresh proof; it does not wait on parent completion or public qualification. Preserve F4.1/F4.2/INT.2 and existing RF ownership/evidence records.
