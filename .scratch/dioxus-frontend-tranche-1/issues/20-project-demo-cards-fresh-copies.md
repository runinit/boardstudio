# 20: Show bundled demo cards and start fresh copies

**What to build:** The Project library and Project menu show the existing REVIUNG41 and Sofle v2 demos as fixture-backed cards. Starting a card opens an editable project copy with a new identity each time.

**Blocked by:** None (the current library, fixture archive/JSON assets, Runtime import path, Session open, and card preview capability are present). The F2.1 parent and full demo-family tickets remain open.

**Status:** implemented in isolated source; combined check and paired browser journey pending

- [ ] Show accessible REVIUNG41 and Sofle v2 demo cards in the library and Project menu, with layout previews and key/board summaries derived from each matching packaged fixture document. A preview load failure uses the established fallback and leaves the card action available.
- [ ] Starting either demo creates a fresh project ID for that fixture copy before the normal Session open. Repeated starts produce independently saveable/listable copies and preserve the fixture's project content and referenced assets.
- [ ] Preserve generic `.boardstudio` archive-import identity semantics. Demo open errors and stale/superseded completions leave the current accepted project usable and cannot replace a newer open.
- [ ] In one paired React/Dioxus browser journey, compare the two card previews/summaries, start each demo, and start one of them again to verify distinct accepted identities. Reuse existing library, archive, persistence, and stale-open evidence for unchanged behavior; record the exact source, fixtures, accepted IDs and browser result.
- [ ] Run the affected combined compile/package check once and retain RF handoff; do not claim full F2.1 or completion of the Sofle, measured-demo, or VIK review-demo tickets.
