# 20: Show bundled demo cards and start fresh copies

**What to build:** The Project library and Project menu show the existing REVIUNG41 and Sofle v2 demos as fixture-backed cards. Starting a card opens an editable project copy with a new identity each time.

**Blocked by:** None (the current library, fixture archive/JSON assets, Runtime import path, Session open, and card preview capability are present). The F2.1 parent and full demo-family tickets remain open.

**Status:** bounded cards, fresh-copy identity, saved listing and reopen are qualified; preview-load fallback, demo-open failure/supersession injection, and the wider F2.1 catalogue remain open. See [candidate receipt](../evidence/project-demo-cards-fresh-copies-34767/RESULTS.md).

- [x] Show accessible REVIUNG41 and Sofle v2 demo cards in the library and Project menu, with layout previews and key/board summaries derived from each matching packaged fixture document.
- [ ] Force an unavailable/malformed preview document and verify the established fallback appears while its demo action remains available. The scoped public journey used healthy packaged fixtures.
- [x] Starting either demo creates a fresh project ID for that fixture copy before the normal Session open. Repeated starts produce independently saveable/listable copies and preserve the fixture's project content and referenced assets.
- [x] Preserve generic `.boardstudio` archive-import identity semantics. `Runtime::import_file` continues to call the shared unpack path without a replacement ID; the unchanged public archive flow is covered by reused import/reopen evidence.
- [ ] Inject a demo archive-load/open failure and a superseded demo completion; prove neither replaces or obscures the current accepted project. The scoped journey did not inject failures; the existing open-sequence admission remains in source.
- [x] In one paired React/Dioxus browser journey, compare the two card previews/summaries, start each demo, and start one of them again to verify distinct accepted identities. Reuse existing library, archive, persistence, and stale-open evidence for unchanged behavior; record the exact source, fixtures, accepted IDs and browser result.
- [x] Run the affected combined compile/package check once and retain RF handoff; do not claim full F2.1 or completion of the Sofle, measured-demo, or VIK review-demo tickets.
