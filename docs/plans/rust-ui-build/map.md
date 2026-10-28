# Map: Build the rust-ui migration

Parent specification: [spec](../rust-ui-migration/spec.md) (decisions in [rust-ui-migration](../rust-ui-migration/map.md)). Every ticket is `ready-for-agent`; blocking edges
decide which can be claimed, not a different readiness label. Published 2026-10-21 and approved by the user as a 26-ticket breakdown.

The work is an expand, migrate, contract refactor on the long-lived branch `feat/rust-ui-migration` ([branch plan](../rust-ui-migration/branch-plan.md)): tickets
001 to 009 add the new layer beside the old `Board*` controls, so every later ticket lands green; 010 to 023 migrate by area; 024 deletes the
old layer once no caller remains; 026 is the single merge to `dev`.

## Tickets

| Ticket | Direct blockers | Delivers |
| --- | --- | --- |
| [001: Walking skeleton: one copied Button, both Themes, one migrated control](issues/001-walking-skeleton.md) | None | The smallest end-to-end path through every layer of the new design. |
| [002: Complete Theme system](issues/002-complete-theme-system.md) | 001 | Both Themes are complete in light and dark: every token the copied components and the shell need, so later tickets only consume them. |
| [003: Base component set](issues/003-base-component-set.md) | 001 | The rest of the rust-ui components the mapping needs are available as copied source, ready for the accessibility and migration tickets: Input, Input Group, Field, Label, Select, Checkbox, Collapsible, Accordion, Toggle Group, Toolbar, Popover, Dropdown Menu, Dialog, Alert Dialog, Tabs, Tooltip, Slider, Combobox, Alert, Callout, Badge, Card, Separator, Kbd, Item, Empty, Spinner, Skeleton, Stepper, Toast and Table.. |
| [004: Vendored Lucide icons](issues/004-lucide-icons.md) | 003 | Icons come from vendored Lucide, one small component per icon, in place of the `icons` crate. |
| [005: Old-CSS check](issues/005-old-css-check.md) | 001 | A repository check reports how many `.board-*`, `.m1-*` selectors and `--wb-*` variables remain, so progress is visible during the migration and the final merge can be blocked on zero.. |
| [006: Accessible Select and Combobox](issues/006-accessible-select-combobox.md) | 003 | Select and Combobox pass every row of the accessibility floor, so the 105 selects in the app can migrate to them.. |
| [007: Accessible action menus](issues/007-accessible-action-menus.md) | 003 | Dropdown Menu is usable for the Project, Settings, Transform and Layout command menus. |
| [008: Accessible Toolbar, Toggle Group, Slider and Toast](issues/008-accessible-toolbar-toggle-slider-toast.md) | 003 | The remaining controls with measured gaps pass the floor.. |
| [009: Dock primitives](issues/009-dock-primitives.md) | 002, 003 | The two building blocks of the new docks exist and work on their own: a non-modal Sheet and an in-flow Sidenav dock with its own resize.. |
| [010: Header card](issues/010-header-card.md) | 002, 004, 007 | The header becomes a 56px borderless card with the project identity and Project menu, the Workspace links, the Command search button, save status, undo and redo, one theme control and Export. |
| [011: Docks and narrow-screen sheet](issues/011-docks-and-narrow-sheet.md) | 009, 010 | Objects and Inspector are cards on the new docks, with pinned and collapsed modes, and below 980px every Workspace shows them as one Sheet with Objects and Inspector tabs, leaving the canvas visible and interactive.. |
| [012: Command palette](issues/012-command-palette.md) | 004, 006, 010 | Ctrl/Cmd+K opens a searchable palette of the workbench's existing actions and objects, as specified.. |
| [013: Toasts](issues/013-toasts.md) | 008, 010 | Transient results of one-shot actions appear as Toasts instead of inline text.. |
| [014: Shell glue sweep](issues/014-shell-glue-sweep.md) | 011, 012, 013 | The remaining shell files have no raw controls and no `Board*` left.. |
| [015: Layout toolbar strips](issues/015-layout-toolbar-strips.md) | 007, 008, 011 | Layout's two fixed strips above the canvas are rebuilt on Toolbar, Toggle Group and Dropdown Menu, with Snap staying in the canvas toolbar. |
| [016: Layout sweep](issues/016-layout-sweep.md) | 006, 015 | The rest of the Layout workspace is migrated. |
| [017: Case settings, part 1: dimensions, construction and profiles](issues/017-case-settings-1.md) | 006, 008, 011 | The first half of the Case mechanical settings: dimension fields, construction controls, profile guidance and the profile geometry editors. |
| [018: Case settings, part 2: mounts, openings, battery, manufacturing and pairs](issues/018-case-settings-2.md) | 017 | The second half of the Case mechanical settings: mounting and mount collections, openings, battery, manufacturing and the paired-feature controls. |
| [019: Case rest](issues/019-case-rest.md) | 006, 008, 011 | The other Case files (bodies, collision checks, viewer controls, layers, panels) and the Case workspace host. |
| [020: Parts library and inspector](issues/020-parts-library-inspector.md) | 006, 008, 011 | The Parts library, previews and inspector. |
| [021: Parts editors](issues/021-parts-editors.md) | 006, 008, 011 | The Parts editors: assembly, model, profile and generator editors, custom definition and footprint import. |
| [022: PCB, Keymap and Keycaps](issues/022-pcb-keymap-keycaps.md) | 006, 008, 011 | The three smaller Workspaces together, with Tabs for the keymap layers. |
| [023: Library and Export](issues/023-library-and-export.md) | 006, 011 | The project library and Export page, the project-delete confirmation as an Alert Dialog, and the setup guide with a vertical Stepper inside its four accordion sections. |
| [024: Delete the old layer](issues/024-delete-the-old-layer.md) | 014, 016, 018, 019, 020, 021, 022, 023 | The `Board*` wrappers and the legacy styling are gone, because no caller remains.. |
| [025: Rewrite DESIGN.md for the new structure](issues/025-rewrite-design-md.md) | 024 | DESIGN.md describes the shipped workbench: the header, docks, cards style, narrow-screen behaviour, controls and Themes, in place of the old Layout and Components sections.. |
| [026: Merge gate and single merge](issues/026-merge-gate.md) | 024, 025 | The migration branch passes the gate and is merged to `dev` once.. |

### Frontier and rationale

- **Initial frontier: 001 only.** It is the tracer bullet through every layer; nothing else builds on unproven dependencies.
- After 001: 002, 003 and 005 can start. After 003: 004, 006, 007, 008 can run in parallel; 009 also needs 002.
- 007 (Dropdown Menu) is the largest unknown: estimate it first. 010 and 012 depend on it only through the menus and palette.
- After the docks (011), the workspace tickets 015 to 023 run at most **3 at a time** (build limits), Layout first.
- 024 waits on every migrate ticket; 025 and 026 follow.
- The existing [editor workspace extraction](../editor-workspace-extraction/map.md) plan is independent: each ticket merges `dev` first and, if
  the matching extraction ticket for the same Workspace is in progress, waits for it.

## Shared contract

Every ticket that touches UI code holds to these, so they are not repeated in each ticket:

- Read the [spec](../rust-ui-migration/spec.md), [mapping](../rust-ui-migration/mapping.md), [theme contract](../rust-ui-migration/theme-contract.md), [accessibility floor](../rust-ui-migration/accessibility-floor.md) and
  [inventory](../rust-ui-migration/inventory.md) sections the ticket links; refresh against the checkout, which changes.
- rust-ui supplies the look and `dioxus-primitives` the behaviour; copied code has no inline script and no unowned document listener.
- Controls are found in tests by accessible role and name, not by `.m1-*` or `.board-*` classes or tag names; behaviour assertions stay the
  same except for decided changes (native `select` and `details` become custom controls, auto-hide removed, the palette and Toasts are new).
  A ticket lists such changes in its outcome and updates the native test-name baseline in the same commit as any rename.
- Every adopted interactive control passes its rows of the accessibility floor in a repository browser test before the ticket closes.
- A lane deletes only the old CSS its own files used; shared rules wait for ticket 024. `presentation.rs` belongs to the shell tickets; other
  tickets touch it only for their own Workspace.
- Run the documented checks appropriate to the change; one cargo or wasm-pack run per `target` directory, through the supported scheduler.
  Put long logs under the gitignored evidence directory and report only results and failures.
- Behaviour that decides what a control does (selection, Undo, shortcuts, precise values) does not change.
- Record in the ticket's outcome: what changed, the commands run and their results, screenshots where the ticket asks for them, and anything
  that revised a decision (then update the source document and say why).

## Lane protocol

How any agent works a ticket here, so prompts only need to point at a ticket and this section.

1. **Worktree.** Never edit in the integration checkout. Create a lane worktree from `feat/rust-ui-migration` per
   [parallel lanes](../typed-core-edits/parallel-lanes.md) (branch `lane/<NNN-slug>`, worktree under `.worktrees/`, `target` linked into
   `~/.cache/boardstudio-target/<lane>`, the ignored `.claude/settings.json` from `.worktreeinclude`, and the `cad/.cache` and `cad/wasm/pkg`
   copies when a build needs them). One cargo or wasm-pack run per `target`; builds and tests lift the sandbox unless the target path is
   allowlisted (AGENTS.md).
2. **Claim.** Set the ticket's `Status: claimed` and commit it on the lane branch before any other change.
3. **Work.** Follow the ticket, its [shared contract](#shared-contract) and the documents it links. Prefer the supported `scripts/check.py`
   steps the change needs (`quick`, `build`, `browser`); put long logs under the gitignored `.evidence/` and report results only.
4. **Resolve.** Append an `## Outcome` section to the ticket (what changed, commands run and results, screenshots where asked, anything that
   revised a decision and the source document updated for it) and set `Status: resolved`.
5. **Merge back, locally.** Merge `feat/rust-ui-migration` into the lane branch, resolve conflicts there, re-run the checks the merge could
   affect, then update the migration branch with a fast-forward only: `git push . HEAD:refs/heads/feat/rust-ui-migration`. If it is rejected,
   merge again and retry. Never push to `origin`, never force, never touch `dev`.
6. **Report** to the coordinator in at most ten lines: ticket, result, final commit, evidence path, follow-ups or risks. Everything else
   lives in the ticket's outcome.
