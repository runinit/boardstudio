# Issue 07 Dioxus generator-settings paired journey

**Status:** bounded representative-value journey passed; malformed structured input and preview-failure recovery remain unverified.

- Dioxus URL: `http://127.0.0.1:34751/`
- Served source: `ab532f275cb14385db3e4e52e5adcb5fb7c3ee18`
- Served source provenance SHA-256: `2b1947a952f895dcee23c795cd99f41366245e6189a7be50a515df4e068053f5`
- Build: `frontend-parts-generator-pcb-context-keymap-inspector-20261003`
- Browser profile/session: `parts-issue07-dioxus-34751-936481b4b15c` (agent-owned)
- Project: `Sofle v2 copy` from the served project picker; project title `Sofle v2` after opening.
- Selected definition: `MX switch`, `ergogen:ceoloide/switch_mx`; bundled source `ceoloide/switch_mx`, package identity `bundled-1`.
- Fixture: bundled Ergogen catalogue definition and parameter schema; no external file.

With the project open, Parts selected the MX switch with Keycap Width 18 mm. Changing the local draft to 20 mm produced the visible `Unapplied generator preview` status and refreshed the footprint preview before Apply. `Apply generator settings` became enabled; after Apply the Inspector showed Width 20 mm and the project showed `Revision 4 · Saved`. Undo restored Width 18 mm and advanced to `Revision 5 · Saved`. Reloading the served app reopened the same project; the Parts Inspector still showed Width 18 mm, with the Apply button disabled and the saved state at Revision 5.

Screenshots:

- Candidate preview before Apply: `/home/chris/.local/share/boardstudio/reviews/parts-generator-issue07-20261003/dioxus-preview-draft.png`
- Accepted Width 20 mm: `/home/chris/.local/share/boardstudio/reviews/parts-generator-issue07-20261003/dioxus-generator-applied.png`
- Reopened after Undo at Width 18 mm: `/home/chris/.local/share/boardstudio/reviews/parts-generator-issue07-20261003/dioxus-generator-reopened-undone.png`

The browser reported no page errors. This journey establishes draft → visible source-backed preview → accepted Apply → one-step Undo → saved reload for the representative numeric field. It does not cover malformed structured values, preview-failure recovery, or the placed-instance terminal-remap branches. The companion [React reference receipt](07-generator-settings-react-reference-20261003.md) records the pinned React project/source and the matching 18 → 20 → Undo → 18 control journey.

RF note: existing Parts generator, retained preview and normal Runtime accepted-edit/history seams remain sufficient; no new refactoring takeaway observed. Issue 06 remains deferred because the pinned imported-definition journey did not mount its static-model Inspector controls and the Parts surface has no mounted model-viewer owner. This evidence does not close F4.3, INT.2, or the F4 parent.
