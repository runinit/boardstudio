# 011: Complete populated-workspace human acceptance

Status: ready-for-human
Type: acceptance
Blocked by: [010: Final technical verification](010-final-technical-verification.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Existing human-review contract](../../rust-ui-build/issues/026-merge-gate.md#human-workspace-review-instructions-still-required)

## What to build

Complete and record the user's review of the verified, populated application
across the agreed Themes, Appearances and widths. Tie final product acceptance
to the actual checked candidate rather than automated screenshots or a scoped
implementation-complete record.

## Acceptance criteria

- [ ] Identify the checked source and packaged artifact from 010 and the
  representative populated projects used for review. Review the actual
  candidate without replacing the user's existing live preview.
- [ ] Review Layout, Case, Parts, PCB, Keymap, Keycaps, Library, the project menu
  and Export in each combination of Original/shadcn Theme, Light/Dark Appearance
  and desktop/mid/narrow width. Use the agreed sizes 1440×900, 1024×900 and 390×844.
- [ ] Keep a per-Workspace/surface checklist with the project, Theme, Appearance,
  viewport, screenshot and result for every combination. Populate the checklist
  with observed results, not assumed passes or empty automated capture slots.
- [ ] Include real Case geometry and a populated 3D canvas. In Layout, inspect
  the recovered placement and scope workflows, split focus, anchor/reference
  feedback, amber previews, spacing ghosts, Snap and nested disclosures.
- [ ] Exercise representative editable fields, menus, dialogs and long Inspector
  content. Review contrast, geometry visibility, overflow/clipping, scrolling,
  keyboard focus, touch targets and selected/disabled states across the matrix.
- [ ] Record the user's explicit approval or reported defects. Automated captures
  support the review but cannot stand in for human acceptance.
- [ ] Keep unresolved defects and incomplete combinations open. If a correction
  changes the candidate, refresh the applicable 010 verification and affected
  human review before final sign-off.
- [ ] Reconcile the acceptance records with the reviewed candidate, actual scope
  and user decision (A8), preserving historical evidence. Do not change or close
  the parent specification or infer permission to push or merge.

## Verification and completion

Human review is the acceptance seam. Agents may prepare the checked candidate,
fixtures, checklist and captures, but this ticket is `ready-for-human` because
the final result requires the user's decision. Preliminary inspection may occur
earlier; final acceptance depends on 010's verified candidate.

## Comments

- 2026-11-06: Published as the final accepted gate. One checklist per Workspace
  keeps review manageable without creating a ticket for every visual combination.
