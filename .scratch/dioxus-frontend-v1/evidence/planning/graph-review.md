# Final dependency graph review

Reviewed `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/tasks.json`, `EXECUTION.md`, `coverage.json`, and the authoritative dependency tables in the workflow specs.

## Material issue still present

**The announced F4.3/F4.4 dependency corrections are not reflected yet.** In the current graph and F4 authoritative table:

- F4.3 `acceptance_after` still contains F4.2.
- F4.4 `acceptance_after` still contains F4.3.

That contradicts the announced cleanup and gates generator editing on custom/imported footprint editing, and the selected-definition preview on the generator form, even though the slices can be exercised against existing saved/catalog fixtures. The Parts preview's F7.3 and INT.2 joins are appropriate for its 3D/lifecycle acceptance; retain those. **Fix:** remove F4.2 from F4.3 acceptance joins and F4.3 from F4.4 acceptance joins in both `tasks.json` and F4's authoritative table. Preserve F4.1 fixture/catalog and F7.1 early viewer-probe start requirements.

## Verified graph corrections and structure

- F3.7 is narrowed to acceptance joins F2.3 and F6C.3; there is no whole-F2 dependency in its normalized row.
- F4.4's 3D completion is now explicitly joined to F7.3 and INT.2, leaving the single shared viewer owned by F7.
- F4.6 starts after F4.1 and joins F3.2 only at completion, matching the distinction between independently authorable/savable assemblies and the F3 placement action.
- INT.2 is a small incremental adapter seam. Consuming tasks can start on fixtures; provider adapters are added as those slices require them. F2/F4/F5/F6/F8 do not wait for an all-provider facade implementation before any UI work.
- F9's inventory, paired verification, AT, compatibility, and applicable performance preparation can start early; final F9.6/9.7 remain gated on complete evidence and explicit production-cutover approval. The external screen-reader gate is recorded rather than waived.
- F8 output-format acceptance joins the owning F3/F4/F5/F6/F7 slices; F8.6 then joins the cross-workflow journeys. F7.8 joins the named viewer consumers, F5 hardware/instance handoff, and F2 shell/panel seam.

A machine check of the current graph found 60 unique task IDs, all references present, `depends_on` equal to the union of `start_after` and `acceptance_after`, a complete matching topological order, and no cycles. I found no other material cycle, false-ready path, or blanket workflow dependency.

**Scope/limits:** Read-only graph/spec review. No repository edits, builds, or browser runs. This is planning review, not implementation acceptance.
