# Project menu name child19 independent source review

Reviewer: independent assigned Sol review. **Standards HOLD; Spec HOLD** for source adoption at the supplied freeze. These findings do not close child19, F2.2, F2.1, F2.4, F9 or INT.2. No source edits or heavy test reruns were performed.

## Exact scope and authority

- Source: `ffaaf8382f1addb0923524f9ee14ab40c308fe8a`; base: `9c92e9fa7886a8b149b24c3f9fe44e71d6249a6a`.
- Evidence: `ed79aef9ce8607f2c197c99656eea93e37fed504`, clean isolated worktree `/home/chris/.local/share/boardstudio/worktrees/project-menu-name-19-20261002` at inspection.
- Diff reviewed: `git diff ffaaf8382f1addb0923524f9ee14ab40c308fe8a^ ffaaf8382f1addb0923524f9ee14ab40c308fe8a -- web/src/presentation/library.rs web/assets/m1.css` (460 insertions, four deletions).
- Contract: `.scratch/dioxus-frontend-tranche-1/drafts/19-project-menu-name-spec.md`, SHA-256 `74cc557a53b29850403601c738e111804ee1aa77b88f1327f9b8f98a18b459d7`; reviewed draft issue SHA-256 `f9883a1178917d55fe4cecb0f97bcc3fda8b481f63ef36d903034a655ed4f98d`; published issue `issues/19-current-project-menu-name.md`. These were read from integration because the isolated source base predates their publication.
- Reference: React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx:478,588–595,1247–1248`, `app/src/ui/project-library.css:34–37,86–95`, and retained `source-audit/react-menu.png`.
- Standards: supplied project AGENTS instructions, `CONSTRAINTS.md`, `docs/agents/issue-tracker.md`, `docs/agents/domain.md`, `CONTEXT.md`, `docs/architecture.md`, accepted ADR 0003, and the code-review smell baseline. No `.codegraph/`, root AGENTS.md or CONTEXT-MAP.md exists in this isolated checkout; applicable supplied instructions and current documented ownership govern. No baseline smell warrants a separate finding.

## Standards

**HOLD — one documented-standard finding.**

**[P2] Preserve the reference field hierarchy and responsive layout.** `web/src/presentation/library.rs:383–389` introduces a 15px `Current project` heading, a separate visible `Project name` label and the input. `web/assets/m1.css:85–87` makes that section a grid at every width. React has one visible `Current project` label beside the input on desktop, then a compact label/input grid below 540px; its accessible name remains `Project name`. The retained reference screenshot and immutable JSX/CSS establish this difference. `CONSTRAINTS.md` requires preservation of theming, control placement and menu hierarchy and comparison against React; a larger three-line stack is an unapproved observable redesign. Match the reference label/field structure and affected desktop/compact styling. This finding concerns the newly owned field, not unrelated existing menu gaps or adjacent export-control ownership.

The private `ProjectNameOwner`/`ProjectNameSubmission` capture, latest-document clone and normal `ReplaceDocument` submission otherwise respect accepted ownership, history, fields, API and visibility boundaries. No new rename store or history implementation is introduced. Existing RF-006/RF-009 remains sufficient; no new distinct refactoring takeaway observed.

## Spec

**HOLD — two findings.**

**[P2] The source changes the required React affordance layout.** The contract requires the menu's `Current project` label/input and React fidelity. The new heading plus additional caption described above changes that hierarchy and lacks the reference desktop-to-compact transition. Repair it before paired visual qualification.

**[P1] Required production completion/persistence/owner regressions are incomplete.** The contract explicitly requires “project/session replacement and late completion,” “persistence/reopen” and failure retaining the accepted name with Runtime feedback. `library.rs:580–606` synchronously substitutes every Persist with `SaveResult::Committed`, ignores terminal effects, and never reads storage. `Runtime::submit:995–1001` exits through the definition-name event-capture override, so this test does not exercise production Runtime settlement/reporting. The reopen at `library.rs:824–833` supplies a newly constructed `ProjectDoc`, rather than reopening the saved renamed document. Moreover, the currently published Runtime owner immediately before that reopen is `replacement`, so the next `menu-name` open changes ID and name; it cannot isolate a same-ID/same-name new-epoch/ABA reset. No delayed prior-operation completion or unmount rejection is exercised. Add bounded production-mounted tests using the real Runtime Session/effect boundary: exact terminal outcome, persistence abort/rejection and accepted-name retention, saved renamed-document reopen, delayed mismatched completion after owner replacement, and same-ID/same-name epoch replacement. Existing private Session request/executor/save-attempt guards look appropriate, but inspection does not fulfill the explicitly required regressions. A rename-specific subsystem is unnecessary.

The retained mounted test does prove the actual Library input's trim/blur/Enter/Escape/blank/unchanged behavior, unrelated-revision draft survival and latest-field preservation, accepted refresh, real Session/CoreEngine Undo/Redo, and different-project reset. Submission captures latest owner/token/revision and rechecks synchronously before dispatch. No source defect in those established paths was found.

## Evidence and remaining joins

Verified byte hashes: normalized mounted log `1c1f8c1fc5705924dab15e19b45199157ce46f2c834f5412771953f410bb5c6d`; retained raw log `9a86addebf00aea711d7722724354d1b3c3823a26cbfa92fa3f9e12b5941b345`; strict WASM Clippy log `19a8bd6af999ab5ab9ae86f180de480b9744b0433e86f600d44d3fe5fc168368`. Mounted Chrome reports 1 passed/114 filtered; Clippy all-targets `-D warnings` reports success. The evidence document additionally reports 19 native web-library tests, fmt and diff checks; native/fmt raw output was not supplied. Independent `git diff --check` passed. Contract/draft SHA-256 comparisons passed. Reused adequate evidence; no build/test rerun.

The expected-red missing-input browser audit is retained and adequate for this feature addition. Author explicitly leaves the paired root-package journey open. Repeat rename, trigger/card/guide synchronization, desktop/compact Light/Dark, actual reload/durable reopen and browser-error checks remain public qualification obligations after source repair, not claimed passes here. No changes to canonical status or unrelated integration work were made.

Summary: Standards 1 finding (worst P2); Spec 2 findings (worst P1). Both axes HOLD at `ffaaf8382f1addb0923524f9ee14ab40c308fe8a` / `ed79aef9ce8607f2c197c99656eea93e37fed504`.
