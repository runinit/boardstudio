# Integrated Inspector browser evidence — 2026-10-02

These retained receipts compare native wheel scrolling in the Inspector for the same Sofle fixture (`/tmp/keycaps-fit-fixture.boardstudio`) on the pinned React reference and the integrated Dioxus candidate. The React inspector content reaches scroll position 288; Dioxus reaches 515. This is bounded scroll acceptance evidence for the Inspector content. It does not qualify edits, Undo/Redo, save/reopen, or the surrounding Keycaps workflow.

The Dioxus root and subpath served candidate is `frontend-inspector-idb-integrated-20261002`, source `71a144ec3a58c8e7fe8169fae5fbe8429f995f6a`, build provenance at `web/target/builds/frontend-inspector-idb-integrated-20261002/provenance.json` (22 commands, 1,291 source hashes, zero source drift, 145 assets per route). Root wheel receipt records scroll position 515; paired React receipt records 288.

Paired React and Dioxus keyboard screenshots record the same Find a key flow: type `left-thumbs-SW5`, then Tab, ArrowDown and Enter; both select `left-thumbs-SW5` in the right pane. This establishes selection/reachability parity only; it does not establish editing or persistence.
