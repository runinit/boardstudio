# Case13 transport admission source review

Reviewed on 2026-10-02 by the independently dispatched Sol 6.1 High reviewer. This is a bounded source review; it does not close Case13 public acceptance or a parent milestone.

## Exact packet

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/case13-wireless-right-20261002`.
- Branch: `codex/case13-wireless-right-20261002`.
- Source commit: `6dfb7fe4b22b74eab878a1c590e3fe302d2bdd0b`.
- Parent/fixed point: `61a81cc2bbaa7546e6e1452d3bf7b3735cd4f365`.
- Diff reviewed: `git diff 61a81cc2bbaa7546e6e1452d3bf7b3735cd4f365...6dfb7fe4b22b74eab878a1c590e3fe302d2bdd0b`.
- Production source blob `web/src/presentation.rs`: `913c3980c0f7f61fbfbf42129f278baefc0d3cf5`.
- Frozen handoff SHA-256: `151f4856c7cc281f202815880b6b6f4812a8b1c281127f84739312a6a095b540`.

The governing scope is the parent-authorized correction of the observed unaccepted Case transport selector operation, the exact handoff, CONSTRAINTS.md, supplied AGENTS instructions, domain/issue-tracker documentation, and existing Case13 battery evidence. No new API, schema, configuration normalization, or general ownership redesign is authorized. Standards and Spec were assessed separately within this bounded reviewer assignment.

## Standards

Clear for the source correction; zero actionable source findings.

The new private `case_setup_context_is_current` helper extracts the Case-specific predicate without widening visibility or adding a new controller boundary. The change removes the selected part-tree dependency from the physical assembly owner. Existing ProjectGuide behavior, operation observer, accepted document replacement, persistence/history pipeline, and controller lifetime remain unchanged. RF-006 is updated in both the prose and machine register with the precise observation and open public gate; no new refactoring ID is introduced.

There is one receipt hygiene correction, separate from the source verdict: `git diff --check <parent> <source>` reports trailing spaces in handoff lines 3–5 (Date, Branch, Base). The handoff's statement that diff checks pass is therefore too broad for the complete frozen commit. The source-only diff check passes. Author was asked to remove those Markdown hard-break spaces and correct/preserve the evidence in a docs-only follow-up.

## Spec

Clear for the bounded source correction; zero actionable source findings.

`case_workspace::clear_tree_part_selection` deliberately clears `adapter.selected_context` when operating through the assembly context. Requiring that unrelated selected tree context to admit the assembly's transport control makes the rendered owner absent, so `PhysicalSetupMount::submit(CaseTransport)` silently cannot send the operation. The correction removes that prerequisite while retaining Case workspace and matching captured scope session/document/board, plus strict instance equality.

The enclosing owner predicate still checks the accepted session/document, generation, and, for strict admission, token/revision and `instance_selection.is_current`. `current_source` requires Ready lifecycle, accepted scene/document revision agreement, a current existing board, and an accepted matching Case scope. The controller compares the captured identity to a fresh identity at event admission and rechecks the strict owner after asynchronous preparation before submitting the existing committed ReplaceDocument edit. These checks continue to reject stale workspace, project, board, instance, revision and generation owners. The ProjectGuide match arm is unchanged.

No physical battery projection/default/normalization logic changes. The public payload evidence supports the narrow diagnosis: the failed candidate transport export remains revision 21/wired and is byte-identical to the previously accepted left-battery archive; the genuinely accepted React wireless/flipped archive remains revision 14/wireless and the QA receipt records successful candidate projection when imported.

## Verification and evidence limits

Independent checks on the frozen clean worktree:

- `cargo fmt --manifest-path web/Cargo.toml -- --check`: passed.
- `git diff --check <parent> <source> -- web/src/presentation.rs`: passed.
- Refactoring JSON parsed successfully.
- Full-commit diff check: the three receipt whitespace lines described above; no source whitespace failure.
- All 21 entries of the retained public QA artifact manifest independently matched their SHA-256s. Manifest SHA-256: `c6af4b7e37ee2ced9f917a12110f84b6234c6fd4159d774c086cadc8bfb67d6a`.
- Candidate pre/post attempted transport archives both match `9d906360d185ccab53957b714647a6978afc137bb28f4d8e9c38bad307f9c8ab` and independently decoded as revision 21/wired. React wireless/flipped archive matches `430657c3d1aafeb719c80988dfc9cf62c99042b5ef009936e4ef9848e8e35ed2` and independently decoded as revision 14/wireless.

The author and coordinator report the focused production-helper browser-WASM assertion failing for the former selected-tree prerequisite and passing 1/1 after the correction, plus strict page/WASM Clippy. Those completed observations are reused; this reviewer did not repeat the heavy/browser checks. The frozen handoff does not currently link raw red/green/Clippy logs, and exact retained paths/hashes were requested from the author. The later stalled/interrupted browser repeat remains explicitly unverified and must not be represented as another green run. The added test covers the production helper's no-part-selection admission; it is not a mounted public selector, persistence or history test.

Fresh public repaired transport admission, accepted archive save/reload, Undo/Redo, and applicable scope-change journeys remain open. Existing wired battery and imported accepted-wireless projection evidence does not close those repair gates. No new refactoring takeaway beyond the correctly recorded RF-006 observation was found.

Source verdict: Standards clear; Spec clear. Receipt correction/evidence retrieval tracked separately; public repair and all parent acceptance remain open.

## Docs-only evidence follow-up

Independently checked follow-ups through `7999b12fbe747789c404bc201f9fd4419f73c729` (preceded by `da029329`): only the receipt and two logs change; production source blob remains `913c3980c0f7f61fbfbf42129f278baefc0d3cf5`. Diff check from the reviewed source through this follow-up passes. The full final packet no longer contains the handoff whitespace issue.

Final handoff SHA-256 is `ba3bfa0fa6c0016352d6d13538b88f2c0864544341e1070505b2f82502520c34`. `owner-green.log` SHA-256 `45ef24ba36f92437a567d5ada3def9b3a7cc0decc8917666109656271b733e5f` shows the focused test passing 1/1 in 0.04s; the author explicitly identifies runner cleanup interruption/exit 130 separately from that completed test result. `clippy-green.log` SHA-256 `8b1fd2394f7b6f177ede882431154a419bd9f404f00add314c912e91a7fc0ec0` preserves the successful Clippy completion line. These minimal preserved output receipts are verified, not represented as a fresh independent rerun. The original red assertion was observed during implementation, but no raw red log was saved; the corrected handoff now says so explicitly. Public repaired transport/save/reload/history qualification remains open.
