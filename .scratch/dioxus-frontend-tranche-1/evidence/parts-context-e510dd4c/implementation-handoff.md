# Objects row semantics correction handoff

Commit: `f65b0c833694d1980ac71caa21d2f932cf785d53`, parent `e510dd4c`.
Worktree: `/home/chris/.local/share/boardstudio/worktrees/frontend-tree-20261002`.
The worktree was clean at `96811f92` and fast-forwarded to the requested integration base before editing. Integration source remained untouched.

One file changes: `web/src/presentation/objects.rs`, 5 insertions and 4 deletions. Existing row div now owns role=treeitem, level, selected and conditional expanded metadata. aria-labelledby points to the unchanged selection-button ID. The selection button resumes native button semantics. Disclosure/button callbacks, native focus, ID, CSS, selection and domain code are unchanged. No public API change.

Existing real paired axe4.12.1 red reports: `/var/tmp/tree-515-a11y.json`, `/var/tmp/tree-react-a11y.json`; both have critical aria-required-children and disallowed button children under their tree. Diagnosis/ranked explanations/acceptance rationale remain in `diagnosis-proposal.md` beside this file.

Executed: owned-file `rustfmt --edition 2024 web/src/presentation/objects.rs`, `git diff --check`; both passed. Final worker status clean. No compiler/build/browser runs during the root build reservation. This is implementation handoff, not a verified accessibility fix or self-approval. Independent Spec/Standards review, rebuilt candidate axe and public control checks, and actual AT verification remain open. Source-preserved handlers do not substitute for runtime verification.
