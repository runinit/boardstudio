# First frontend tickets — implementation authorized

The reviewed twelve tickets are published in `issues`; original drafts below are
preserved as planning history. The user authorized implementation and automatic
follow-on ticket creation on 2026-10-02. See [authority](AUTHORITY.md) and
[current execution](execution.json). The full62-item graph continues to govern
scope and parent acceptance.

# Proposed first frontend tickets

**First wave only: 12 draft child tickets beneath four of the 62 existing work packages.**
**Status: draft, awaiting your review of granularity and blockers.**

| Planning level | Count | Meaning |
| --- | --- | --- |
| Full frontend work packages | 62 | All remain in the unchanged parent graph |
| Parents expanded in this proposal | 4 | INT.1, F2.1, F2.3, F3.1 |
| Draft child tickets for those parents | 12 | The numbered list below |
| Other parents awaiting child-ticket decomposition | 58 | Still planned; not removed or replaced |

See the [full backlog and parallel execution view](../dioxus-frontend-v1/PARALLEL-EXECUTION.md).
The numbered list is a dependency-respecting index, not a serial execution order.
There is no requirement to finish all twelve before unrelated work starts.

This applies `to-tickets` to the first runnable tranche from the existing
`to-spec` documents: INT.1, F2.1 project library, F2.3 panels/drawers and F3.1
Layout tree/selection. The other frontend phases remain in the existing roadmap.
The existing parent issues and 62-task graph are unchanged.

Each draft below has its own behavior, real integration and acceptance checks.
Only ticket 01 is a preparatory refactor. No ticket is published as
`ready-for-agent`, and no new frontend implementation has started.

1. **[Isolate the existing workspace UI for parallel work](drafts/01-private-workspace-composition.md)** — **Blocked by:** None. **Delivers:** The existing library, Objects panel and Inspect panel run through small private page components while the current editor continues to behave the same. This is the bounded prefactor before the visible tickets.

2. **[Browse and open saved keyboards](drafts/02-saved-keyboards.md)** — **Blocked by:** 01. **Delivers:** Your keyboards displays usable saved-project cards and opens the chosen keyboard through the existing storage/session flow.

3. **[Find saved keyboards by name](drafts/03-saved-search.md)** — **Blocked by:** 02. **Delivers:** Users can search the saved-keyboard cards, clear the search and open a matching result.

4. **[Start fresh Sofle demo copies](drafts/04-sofle-demo-copies.md)** — **Blocked by:** 02. **Delivers:** The demo section offers Sofle v2, RGB and Choc as real editable copies with the reference card previews and summaries.

5. **[Open all measured-layout demos](drafts/05-measured-demo-copies.md)** — **Blocked by:** 04. **Delivers:** Every measured keyboard in the reference demo catalogue is visible and starts a faithful editable project.

6. **[Open the VIK module review demo](drafts/06-module-review-demo.md)** — **Blocked by:** 04. **Delivers:** The VIK module review demo starts as an editable project that preserves its above/below mounted-module configuration.

7. **[Pin, auto-hide and collapse desktop panels](drafts/07-desktop-panel-modes.md)** — **Blocked by:** 01. **Delivers:** Objects and Inspect support the reference desktop panel modes and remember their preferences.

8. **[Resize desktop panels with pointer and keyboard](drafts/08-panel-resize.md)** — **Blocked by:** 07. **Delivers:** Users can resize Objects and Inspect while keeping the canvas usable and retaining the chosen width.

9. **[Use compact Objects and Inspect drawers](drafts/09-compact-drawers.md)** — **Blocked by:** 07. **Delivers:** At compact widths, users can open, interact with and dismiss Objects and Inspect without losing keyboard focus.

10. **[Browse and select the Layout object hierarchy](drafts/10-layout-object-tree.md)** — **Blocked by:** 01. **Delivers:** Objects shows the active canonical board’s matrices, row/column groups, keys, cell components and standalone components with synchronized canvas selection.

11. **[Select outline versions and locate bridges](drafts/11-outline-tree-navigation.md)** — **Blocked by:** 10. **Delivers:** The Layout tree exposes the active board’s outline, Generated and saved versions, and applicable bridges; users can activate a version and locate a bridge in the real canvas.

12. **[Correct rectangular range selection and modifiers](drafts/12-rectangular-selection.md)** — **Blocked by:** 10. **Delivers:** Shift selects the reference rectangular key range, while Ctrl/Cmd toggles and tree/canvas selection remain consistent across scope changes.

## Agents and delivery order

Start with ticket 01 using Luna High and a dedicated Astra High contract review.
After it is accepted, saved keyboards (02), desktop panels (07) and Layout tree
(10) are independent lanes. The full graph also unlocks Parts, PCB, Keymap,
Keycaps, Case, Export and shared adapter work; these do not wait for this entire
tranche. Expand and review their next bounded tickets before dispatch as capacity
frees, prioritizing useful frontend behavior over finishing one lane end to end. Use two Luna authors plus one Luna verifier; stagger
the third lane and rotate a slot to Astra review. Shared shell/style/runtime
edits remain coordinator-owned and serialized.

The saved search (03) uses Luna Medium. New async/selection/focus/gesture work
uses Luna High. Ticket 12 is an identified behavioral repair, so Astra High owns
it, with a different independent reviewer. Demo ticket 04 also sends any confirmed
existing copy-identity bug to Astra rather than asking Luna to guess at repairs.
Use Extra High only for an unresolved material question or difficult bug.

After 02, search (03) and the first demo family (04) are independent. After 04,
measured demos (05) and VIK (06) are independent. After 07, resize (08) and drawers
(09) are independent. After 10, outline navigation (11) and range repair (12)
are independent. File overlap is coordinated without adding false product edges.

Exact file ownership, interfaces, fixtures and affected commands are attached at
dispatch; they are not frozen in ticket prose. See [shared acceptance](ACCEPTANCE.md),
[machine proposal](proposal.json), [source decisions](SOURCE-NOTES.md) and the
existing [model policy](../dioxus-frontend-v1/AGENT-ROUTING.md).

## Coverage and limits

| Existing parent | Tickets covering its first-tranche behavior |
| --- | --- |
| INT.1 | 01: reviewed private composition, one session, preserved F1/F3a |
| F2.1 | 02: saved cards/list/error/open; 03: search; 04–06: all 19 reference demos and fresh copies |
| F2.3 | 07: modes/preferences/visibility; 08: pointer/keyboard resize; 09: compact drawers/focus |
| F3.1 | 10: board/matrix/row/column/key/component tree; 11: outline/version/bridge rows and actions; 12: toggle/rectangular range parity; scope correctness applies throughout |

Every ticket includes affected visual, keyboard, runtime and state checks. Final
combined evidence must also exercise open project → select in tree → resize or
open/close panels → edit/Undo → save/reopen without losing scope or visibility.
This reuses each ticket’s evidence and covers the affected joins; it is not a
separate catch-all ticket that postpones integration.

New/delete/import/portable-copy flows, setup guide, full Layout authoring,
Parts/PCB/Keymap/Keycaps/Case/Export completion and release qualification retain
their existing tasks. This tranche does not complete those milestones or
production cutover. Applicable blocked gates, including actual assistive technology,
remain visible; approving tickets does not waive them.

## Review requested

Does this granularity feel right, do the blocking edges represent real prerequisites,
and should any tickets be merged or split? On approval, publish the exact accepted
breakdown as one numbered `ready-for-agent` Markdown file per ticket in this
tranche’s `issues` directory. The parent issues stay unchanged.

The invoked `to-tickets` skill explicitly says: “Iterate until the user approves
the breakdown.” Drafting and checking this concrete proposal comes before that
approval; approval is the remaining step before ticket publication.
