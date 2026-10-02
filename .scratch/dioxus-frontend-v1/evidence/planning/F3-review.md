# Independent review: F3

## Material finding

**F3.7 has an unnecessarily broad F2 milestone dependency.** `F3.json` makes integrated Layout parity depend on the whole `F2` milestone, and `F3.md` repeats `F2` as a prerequisite for integration. F3.7's journey needs shared shell/panel integration and public acceptance, but it does not use unrelated F2 project-library lifecycle, archive, or setup-guide functionality. F2 itself splits those responsibilities into separate slices (notably F2.1/F2.2 versus F2.3/F2.4). This can hold Layout qualification behind work with no execution dependency, contrary to the plan's stated goal of avoiding whole-milestone blockers.

**Concrete correction:** Replace the F3.7 dependency on `F2` with the narrow shared-shell/panel task(s) actually required (currently F2.3 for panel integration, plus F1 shell if needed); keep the rest of F2 outside the F3 task graph. In `F3.md`, say integrated acceptance waits for the shared shell/panel handoff, not all F2 work.

**Evidence:** `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/workflows/F3.json`, task `F3.7.depends_on`; `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/issues/03-layout.md`, prerequisite text and F3.7 row. `/tmp/boardstudio-workflow-plans/F2.json` separates library/lifecycle from panel/guide slices. The F3 public journey lists Layout editing, findings, view switch, save/reload, and history; it does not exercise F2 library or archive workflows.

## Checks with no additional findings

The F3 source boundary descriptions consistently distinguish existing session/core geometry operations from missing Dioxus adapters and the shared F7 viewer; the plan preserves canonical Layout-board scope and avoids making F7 Case physical-instance work a dependency. The scripting story is bounded to the observed React editor. The recorded drag-threshold mismatch and F3.8 addition reflect the inspected source and the prior coordinator corrections.

**Scope/limits:** Planning review only against baseline `c827c4e6` and pinned React `5a472a94`. No builds, browser runs, or repository/draft edits. This report is not implementation acceptance.
