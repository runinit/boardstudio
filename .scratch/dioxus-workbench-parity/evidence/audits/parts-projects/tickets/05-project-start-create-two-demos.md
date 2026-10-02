# Draft child ticket: Project start, new keyboard, and two starter demos

Parent: F2.1 Project start, library, search and demos. Root owns shared mount/project lifecycle adapters; feature author owns only the private project-start leaf/controller contract approved by root.

## Goal

Give a new user a public Project start route that can create an editable keyboard through the existing setup guide, open saved entries, and start two requested real fixtures: Sofle v2 and REVIUNG41.

## Start / acceptance edges

- Start after INT.1’s project route and private immutable/card/action contract is reviewed.
- Use existing BrowserStore list/load and fixture open operations; the user-visible new project path relies on the existing Session/open/edit flow and no parallel project/session store.
- F2.2 portable archive, durable copy/open/delete/recovery/error/race behavior remains a separate acceptance join to INT.2. Do not delay the basic visible project-start card on completion of unrelated workspace panels or F3 authoring.
- Two fixtures are initial scope only. Preserve full F2.1 gallery acceptance independently; do not describe two demos as 18-demo parity.

## Behavior

- Render saved/current keyboard cards with the existing preview/count/current-state projection and clear loading, empty, no-match, damaged-preview, list-error and retry states.
- Expose New project/Create new keyboard; creating opens the actual existing setup guide, beginning at name, one/split physical assembly and reversible-layout choice.
- Expose exactly the first two requested, existing fixtures: Sofle v2 and REVIUNG41. Each action opens that fixture via the existing Runtime/Session and preserves a unique independent project identity; do not fabricate fixture documents.
- Expose saved open and `.boardstudio` import where existing action ports are available. Browsing/search/panel open do not switch the active project or mutate revision/history.
- No rename/duplicate feature unless pinned React source is extended; it currently has neither as a project-library action.
- Root retains shared shell/menu/CSS; no public API, storage schema, or Core lifecycle changes in this child.

## Acceptance

- Paired public browser journey: start screen → create blank → setup step 1 → open Sofle v2 → return to library → open REVIUNG41 → verify names/project IDs and saved cards are distinct; search saved list and test no-match/clear.
- Verify browsing does not mutate the active project, open action changes identity once, stale list/open results cannot supersede a newer choice, and preview failures are recoverable.
- Test React-produced `.boardstudio` import when the F2.2 private archive port is integrated; do not gate this initial child on archive download/delete race acceptance.
- Preserve F2.1 full gallery/count/current-marker acceptance as a separate final qualification record. Root owns route integration and shared styles.
