# React reference public behavior observations

Source checkout identity supplied by coordinator: React reference `5a472a9426e6e38993361da402cd4ec730feb369`; browser `http://127.0.0.1:5173/`, loaded real REVIUNG41 demo. My automation checkout is `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001` at `5e5d3370f9e5d44c8dd50ad26570f2cd86f5dafb`. `git diff` for ProjectLibrary/WorkspacePanel/Workbench is empty; existing unrelated tracked migration ledger edits were present. Browser viewport 1280x577.

## Saved keyboard entry point

Entry screen has `Your keyboards 0`, region `Your keyboards`, search box `Search saved keyboards`, `Create new keyboard`, and 19 demo cards. Accessible action names are `Start <keyboard name>`; `Start REVIUNG41` enters the actual editor. The editor displays `REVIUNG41`, `Saved locally`, and the browser-save notice. The isolated browser context initially had zero saved project cards; starting REVIUNG41 created one genuine saved record, characterized below.

## Panel modes

Editor initial left Object panel and right Inspector panel are visible. DOM separator widths/positions at default viewport: Objects separator x=230, width=8 (panel visible width approx 235); Inspector x=957, width=8 (right panel visible width approx 323). localStorage meanings observed: `boardstudio:v2:panel:left` = `{mode:"autohide",width:null}` after my own interaction; right = `{mode:"pinned",width:null}`. Before interaction the left was initially pinned as expected; I toggled it to autohide while characterizing. Do not use these stored values as the pristine default oracle.

Each options menu has `Auto-hide objects` / `Collapse objects` (and matching inspector labels). On selecting autohide, its panel remains revealed while pointer is over panel/menu; moving pointer to canvas and waiting 500 ms hides panel content and exposes a left rail button `Show objects` (its text is `Objects`). Clicking rail reveals it immediately and starts another idle cycle. While hidden, panel controls/tree are removed from accessibility snapshot. Inspector menu exposes `Auto-hide inspector`, `Collapse inspector` independently. `Objects options` and `Inspector options` have expanded state and menu dismissal needs further capture.

## Existing axe baseline

At loaded REVIUNG41, axe reports 2 violations: `aria-required-children` at `.wb-tree-viewport` role=tree containing disclosure buttons, and `page-has-heading-one`. Axe also reports incomplete ARIA and color contrast checks. Treat as pre-existing reference findings; this is not candidate evidence.

Screenshots: `/tmp/frontend-run/ui-verifier/reference-start.png`, `reference-autohide.png`, `reference-panels-initial.png`.

## Saved card from live session

Starting the REVIUNG41 demo created a genuine saved-project record in this agent-browser session. Opening the Project menu showed one entry card, accessible action `Open REVIUNG41` plus `Delete REVIUNG41`, and card content `REVIUNG41`, `41 keys · Single board`, `Current`. The browser DOM's rendered text showed the card summary and it is within the same `Your keyboards` region at entry and in the project menu. A real click on `Open REVIUNG41` returned to the editor. This is sufficient evidence that both library entry points use the same listing and open action for this one record. Screenshot `/tmp/frontend-run/ui-verifier/reference-saved-card.png`.

## Collapse

Options menu has `Collapse objects`, separate from `Auto-hide objects`. Clicking Collapse sets left preference to `{mode:"collapsed",width:null}`, hides panel content/controls from the accessibility tree, and exposes the `Show objects` rail. Resize separator moves from x=230 to x=-5; right inspector separator remains x=957. That is a left-width collapse, distinct from autohide's initially full-width overlay plus delayed idle hide (the mode value changes). Screenshot `/tmp/frontend-run/ui-verifier/reference-collapse-objects.png`.

Before collapse characterization, I returned both panel preferences to `{mode:"pinned",width:null}` in this isolated browser context and reloaded. The final collapse interaction then changed the left preference to `collapsed`; the demo-created project remains in this isolated context.

## T1-10 object tree

Opening `right keys` revealed two Bridge contexts, six Columns each summarized as `3 keys`, plus the U1 and RST standalone part rows. Each matrix/layer disclosure has its own button; controls are `Expand/Collapse <label>`. Selecting `Column 1` marked that treeitem selected, changed the canvas context tool label to `Select: Column`, and changed Inspect heading to `right keys · Column 1`; Inspector showed column properties and `6 keys selected`. Undo/Redo remained disabled after this selection-only action. This visible 3-key label versus 6-key inspector summary is a reference quirk to preserve or get explicit clarification for, not to silently “correct.” Screenshot `/tmp/frontend-run/ui-verifier/reference-expanded-tree.png`.

Opening `Objects options`, then Escape, closed the menu (`expanded=false`) and restored active focus to the `Objects options` trigger. Both stored mode preferences were still pinned throughout. This is one keyboard focus-return path.

## List order, search and project switch

From an initially empty browser context, I created REVIUNG41 and Sofle v2 by starting those real demos. The settled library places the current project first and then lists remaining saved projects by name: when Sofle v2 was active the sequence was Sofle v2 (`aria-current="true"`), REVIUNG41; after opening saved REVIUNG41 it was REVIUNG41 (`aria-current="true"`), Sofle v2. Both settled states were captured with topbar/editor title and `boardstudio-v2-active-project` ID. Search `REVI` filtered saved entries to REVIUNG41; demo records remained in a separate region. The list briefly displayed `Loading saved keyboards…` then resolved to two records. Clicking saved `Open REVIUNG41` switched active project ID from `4a86e26d-e9e2-48dd-b927-0042e45ed9e5` to `adeea041-18ed-4d7b-9402-fefa1af684ac` and moved the Current marker with it. JSON DOM observations: `/tmp/frontend-run/ui-verifier/current-order-sofle.json`, `/tmp/frontend-run/ui-verifier/current-order-reviung.json`. Screenshots: `/tmp/frontend-run/ui-verifier/reference-saved-order-sofle-current.png`, `/tmp/frontend-run/ui-verifier/reference-saved-order-reviung-current.png`.

Cleanup: the initial isolated browser context had no saved records. Both records were test-created by starting demos, then I closed the named `frontend-ui-reference` browser session at the end of characterization. No production user profile/project was attached to this session.
