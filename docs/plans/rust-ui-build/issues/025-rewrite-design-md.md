# 025: Rewrite DESIGN.md for the new structure

Status: resolved
Owner: lane/025-rewrite-design-md (.worktrees/lane-025-rewrite-design-md)
Type: build
Blocked by: [024: Delete the old layer](024-delete-the-old-layer.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [theme contract](../../rust-ui-migration/theme-contract.md), [shared contract](../map.md#shared-contract)

## What to build

DESIGN.md describes the shipped workbench: the header, docks, cards style, narrow-screen behaviour, controls and Themes, in place of the old Layout and Components sections.

## Acceptance criteria

- [x] DESIGN.md's Layout, Components and token sections match the shipped shell and the theme contract, with no contradiction about where Workspace navigation lives
- [x] README commands and the glossary are checked and updated if affected; the documentation link check passes
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Current

Resolved; see [Outcome](#outcome). Combined review is deferred to Phase 1 close-out.

## Outcome

Rewrote DESIGN.md for the shipped header navigation, pinned/collapsed docks,
narrow non-modal Sheet, cards, controls, and Original and shadcn Themes. The old
record contradicted itself about Workspace navigation and described auto-hide,
a Layout-only sheet, outlined rectangular panels and Snap in Objects; the rewrite
reflects the implementation. The setup guide is a separate overlay, and the 3D
palette depends on Appearance rather than Theme. The theme contract's later
borderless-card decision supersedes its earlier bordered-header wording.

A careful source check also corrected the rewrite's selection-token claims:
default Toggle Group items use `background`, and some existing tree rows, setup
controls and 2D geometry use `selection` rather than `selected`. This existing
usage differs from the theme contract's text-only intent for `selection`; the
document records the exception without changing UI behavior or the contract.
Input and Combobox text highlights use `primary`. The token-usage discrepancy is
for the Phase 1 close-out review, not an adopted change to the contract.

README commands and glossary Theme, Appearance, Workspace and Dock terms were
checked against entrypoints, available flags, checks and shell code; no update was
needed. `python3 scripts/check.py --list` and `python3 scripts/build-web.py --help`
confirmed the relevant commands and flags. `python3 scripts/check.py repo` passed
with the documentation link check (571 Markdown files), registration, ADR and
old-CSS checks: run 1's `.evidence/025/repo-final.log` and the accuracy-fix rerun
`.evidence/025/repo-finish.log`; the resolved-ticket rerun is
`.evidence/025/repo-resolved.log`. `git diff --check` passed. These are
documentation-only changes; no build or browser test was run.

Per coordinator ruling, the combined Spec+Standards review is deferred to the
Phase 1 close-out review, which also covers ticket 026's whole-branch review.
