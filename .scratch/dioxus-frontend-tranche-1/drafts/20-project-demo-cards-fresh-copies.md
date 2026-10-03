## Problem Statement

The Project library currently exposes two bundled demos, REVIUNG41 and Sofle v2, as plain buttons. People cannot inspect their layouts or key/board summaries before opening them. Reopening a bundled demo also imports its archived project ID unchanged, so two starts do not create independently editable saved copies.

## Solution

Show the two currently mounted bundled demos as accessible cards in the Project library and Project menu. Derive each card’s keyboard preview and summary from the same packaged fixture document used by the demo. Opening a card imports its existing archive into a fresh project identity and uses the normal Session, history, and browser persistence flow.

This is a bounded first pair of demos. It does not complete the wider F2.1 demo catalogue or close the existing Sofle/measured/module-review tickets.

## User Stories

1. As a keyboard designer, I want to see a REVIUNG41 card with its actual layout preview and key/board summary, so I can identify it before starting a copy.
2. As a keyboard designer, I want to see a Sofle v2 card with its actual layout preview and key/board summary, so I can identify it before starting a copy.
3. As a keyboard designer, I want a card action labelled as starting that demo, so I can distinguish a new demo copy from opening a saved keyboard.
4. As a keyboard designer, I want to start the same demo more than once and receive a new project identity each time, so edits and saves remain independent.
5. As a keyboard designer, I want a failed demo load or a superseded request to leave the currently accepted project usable, so an incomplete open cannot replace my work.
6. As a keyboard designer, I want fixture-preview failure to leave the demo action available, so I can still try opening the bundled source.

## Implementation Decisions

- Keep the two existing bundled fixture actions and their archive contents as the source of demo projects. Reuse the packaged project JSON for card preview and summary; do not construct a project, run a generator, or perform a CAD operation merely to browse the cards.
- Reuse the existing keyboard card preview geometry and summary conventions for both library entry and Project menu presentation.
- Add a fixture-copy identity choice to the existing private Runtime archive-import path. Only bundled demo starts receive a newly generated project ID after archive unpacking and before the normal Session open. User-selected archive imports preserve their imported project identity semantics.
- Continue to use the existing open-sequence guard, Session, accepted edit/history path, assets, and persistence acknowledgement. A stale completion cannot publish an older demo after a newer open.
- A missing preview document is a card-level fallback; it does not disable the underlying demo action.
- Keep this UI composition and adapter page-private. Do not widen library-crate or public API visibility.

## Testing Decisions

- Compare the same two fixture-backed cards and start actions in pinned React and Dioxus public browser journeys. Confirm each preview and key/board summary comes from the matching project, then open the same demo twice and observe distinct accepted IDs and independently listed saved copies.
- Confirm one representative edit and Undo continue to use the normal accepted Session path; use existing save/reopen evidence where unchanged.
- Keep a focused regression for the fixture-only identity remap so generic archive imports retain their existing identity behavior. Do not add a broad UI test matrix for this reversible card slice.
- Reuse the prior library, archive, persistence, stale-open, and browser packaging evidence where its behavior is unchanged. The coordinator runs the affected combined compile/package check once.

## Out of Scope

- Completing all Sofle variants, the fifteen measured layouts, or the VIK module-review demo.
- Reworking saved-project listing, saved-project search, create/import controls, portable-copy behavior, or Project menu navigation.
- Editing the bundled source fixture or copying React presentation/runtime code into Dioxus.
- Completing the F2.1 parent or closing the existing broader demo-family tickets.

## Further Notes

Current source characterization: the page Runtime fetches the two fixture archives and sends their unpacked `ProjectDoc` through `Event::Open`, while the archive helper is also used by user-imported `.boardstudio` files. The fixture preparation path already emits matching project JSON assets for preview. The TypeScript reference renders its demo catalogue as cards and derives previews from the fixture/demo data without opening a full project during browsing.
