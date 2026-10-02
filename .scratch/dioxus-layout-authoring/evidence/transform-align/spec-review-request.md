# Spec review request — Layout Transform / Align children

Review the source and browser packet at:

- [F3.3 addendum spec](../../specs/F33-transform-align-toolbar.md)
- [Source/browser inventory](source-and-browser-inventory.md)
- [Draft F3.3b matrix Transform ticket](../../drafts/07-matrix-transform-tools.md)
- [Draft F3.3c Align ticket](../../drafts/08-align-selection.md)
- [Draft F3.3d command-pill integration ticket](../../drafts/09-layout-command-pill.md)
- [Proposed child graph](proposed-child-graph.md)
- [RF-005 handoff draft](rf-handoff.md)

Pinned source is React `5a472a9426e6e38993361da402cd4ec730feb369`; candidate source read is integration `42b9fdefe1941e3441ed7d43d2757e93e3588bde`. Browser evidence is clearly separated into same-archive paired control layout versus fresh Sofle v2 menu-only exploration.

Please assess: (1) accuracy of Transform/Align action labels, enabled/disabled scope and resulting edits against React source; (2) whether F3.3b/b/c/d scopes are distinct from F3.3a, matrix structural inspector, and F3.5 Relationships; (3) whether the proposed capability starts are sufficient without implying parent acceptance; (4) whether alignment's private courtyard/keycap AABB policy is an implementable exact policy without widening public/member visibility, guessing unknown geometry, or duplicating a broader CAD algorithm; (5) whether missing standalone part rotation is correctly called out rather than assigned an unsupported adapter; (6) whether root-owned admission, gesture transaction, composition and source/build evidence obligations are precise; and (7) whether any acceptance scenario could pass with a no-op or stale target.

This is a planning review request only: no implementation, Cargo, UI edit journey, public API change, parent/task graph update or RF ledger write is being proposed in the packet.
