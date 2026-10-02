# 02: Resolve packaged Ergogen model IDs to offline raw bytes

**Parent:** F2.2 — Project lifecycle and portable archive (`.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`). This capability is owned by the Parts/Project stream and adds no top-level workflow edge.

**What to build:** A private Rust page provider resolves React-compatible bundled Ergogen asset IDs and metadata, then returns the exact original static model bytes from the deployed Dioxus app at root and subpath URLs, including offline use.

**Blocked by:** None for the private provider preparation. The parent F2.2 workflow retains its existing F2.1 start and INT.2 acceptance joins. This ticket proves only the provider prerequisite and does not assert F2.2 or Issue 16 can close.

**Status:** ready-for-agent

- [ ] Enumerate the same recursive vendor `3d_models` model formats as the pinned React catalogue. Each supported source file has exactly one stable React-compatible `ergogen:model:<vendor>/<filename>` identity, reference filename, media type, original source path, and manifest-safe emitted static path; reject unknown IDs without deriving arbitrary paths or fetching remote URLs. Stage with deterministic opaque filenames safe for the existing offline-path validator; never place encoded or raw URL-sensitive original paths in the offline manifest.
- [ ] Preserve these exact React aliases: `THQWGD001-rotation.stp` → `THQWGD001 #1.stp`, `THQWGD001C-2pin.stp` → `THQWGD001C [2pin] #1.stp`, and `THQWGD001C-4pin.stp` → `THQWGD001C [4pin] #1.stp`. Resolve right-hand IDs/names to left-hand sanitized vendor source files.
- [ ] Stage or serve source model files as static assets outside the page WASM. Root and `/boardstudio/` builds retrieve them through the deployment-prefix-aware provider; staging output paths match the provider metadata exactly and are included in the offline manifest. Feature author owns catalogue generation and a narrow staging helper. The integration coordinator owns invoking that helper in `scripts/build-m1.py` and retaining vendor-source inventory hashes/provenance.
- [ ] Return source bytes unchanged. Provider/archiver transport accepts empty and content-unrecognized bytes; it does not parse model format or use renderer-only `VerifiedModelBytes`. Do not invent expected catalogue digests; capability 01 computes packed hashes and Core checks path/payload equality.
- [ ] Ensure every safe provider path is included in the release offline manifest and cached as a normal application asset. Offline retrieval returns the same bytes without vendor network access; do not relax existing normalized-path validation.
- [ ] Add a deterministic catalogue test for one-to-one source/ID/path mapping and unique IDs/paths, including spaces and URL-fragment filename escaping. Test exact bytes on representative ordinary and alias files; unknown/malformed IDs fail without an HTTP request.
- [ ] Build/serve root and subpath candidates and compare fetched bytes with the source files; verify provider files remain separate static assets (not page-WASM payload), stable safe paths match catalogue entries, and the offline manifest covers them. This does not close archive round-trip, renderer decode, or parent acceptance.
- [ ] Preserve F2.2’s F2.1 start and INT.2 acceptance joins, F8.6→F2.2, issue16's dependency on capability 01, and RF ledger ownership. Record no new refactor takeaway unless implementation evidence shows one.

**Capability handoff:** Capability 01 depends on this provider for stable ID/metadata/raw-byte resolution. Capability 01 introduces and tests its own private `embed_used_models: bool` input; no callable bool is assumed to exist yet. F8 later wires its visible checkbox to that shared preference as paired acceptance. Issue16 continues to depend on completed capability 01, not directly on this provider ticket, and no F2/F8 parent graph edge changes.

**Implementation references:** React source baseline `5a472a9426e6e38993361da402cd4ec730feb369`; source observations include `app/src/bundledModels.ts`, `app/src/storage.ts`, `scripts/build-m1.py`, `web/build.rs`, and `web/src/runtime.rs::resource_url`. These identify the oracle/build boundary; they are not permission to change public APIs or the F2/F8 parent graph.
