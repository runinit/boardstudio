# 023: Library and Export

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

The project library and Export page, the project-delete confirmation as an Alert Dialog, and the setup guide with a vertical Stepper inside its four accordion sections. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The library, Export, firmware export and setup-guide rows of the inventory have zero raw controls and zero `Board*` uses
- [ ] Deleting a project uses an Alert Dialog that starts on the safe action and returns focus; the setup guide's current and ready steps use Stepper and stay freely navigable
- [ ] Existing tests pass after the rewrite rule; Export blockers stay inline
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches


## Implementation journal

Binding verified via T3 `t3_worktree_status`: attached child worktree
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-b3957019`, branch
`feat/library-export-ticket023`; initially clean at `b237903705244a266b07a64e5f23b8a3b5219c03`.
Fast-forwarded committed local `feat/rust-ui-migration` to
`c032d902ba3ae24c20ea588e5349fbe1e9559af5` before claim/setup. Parent dirty docs
were neither copied nor touched. Claim commit precedes implementation.
Exclusive new target: `/home/chris/.cache/boardstudio-target/023-library-export-lineage`;
ignored settings and CAD caches copied using the established lane setup, no new lane.

Pre-agreed seams (before tests):
- Mounted actual Library controls and Runtime owner callbacks: catches deletion
  identity/Cancel/safe initial focus and rename/demo lifecycle regressions; misses
  packaged CSS and trusted keyboard behavior.
- Mounted actual Export/firmware rows with real local owner callbacks: catches
  eligibility, inline blockers and operation outcomes; misses production providers.
- Mounted ProjectSetupGuide with owner stage/name callbacks: catches current/ready
  composition, free navigation and draft continuity; misses packaged responsive CSS.
- Packaged page role/name workflows: catches trusted keyboard, overlay focus return,
  Ctrl+K integration, popup clipping, touch targets and Theme styling; misses screen
  readers, native desktop and global CAD/human/final merge acceptance.

Scope reaches Library/KeyboardCard/DemoActionCard and owned tests, ExportWorkspace/
ExportRowView/ZmkFirmwareExportRow and test-only ExportPanel, ProjectSetupGuide and
its Matrix focus integration, plus only their owned legacy CSS. Preserve 012 tool
request/name ownership, 013 outcome classification, 016 controls and stage/draft
owners, 017 mechanical settings, 020 Parts and canonical renderer unchanged.
