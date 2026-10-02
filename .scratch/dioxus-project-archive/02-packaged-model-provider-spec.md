# F2.2 capability spec — packaged Ergogen model-byte provider

**Status:** Draft for independent Spec and Standards review. This is a bounded private input capability for F2.2 archive packing; it adds no top-level F2/F8 workflow edge and does not change the 62-task graph.

## Problem Statement

The React application can resolve every supported bundled Ergogen model identity to its packaged source filename and retrieve the exact raw file bytes offline. The Dioxus application currently has no such catalogue provider. Its renderer reports bundled model references as missing, and the renderer's verified-byte type rejects empty files and applies a stricter size bound than the archive contract. Consequently the portable archive packer cannot include referenced bundled assets without confusing renderer validation with archive-byte transport.

## Solution

Provide a private Rust page capability that maps stable `ergogen:model:<vendor>/<filename>` IDs to catalogue metadata and retrieves the exact packaged model bytes through a static application asset URL. Stage the vendor model files as static assets, outside the page WASM binary, so models remain fetchable from the deployed root or subpath and are covered by the existing offline asset manifest. Preserve the existing filename aliases and media-type mapping from React. Treat bytes as opaque; the archive consumer computes the digest for the packed asset record/path and lets Core validate that path against the payload.

F2.2 owns this byte source capability and the shared private pack adapter. The capability does not add the Project-menu action. F8 retains ownership of its visible “Embed used models” control and later connects its one shared preference. Ticket 01 will own its private boolean input; neither this provider ticket nor this prerequisite requires the F8 control. Keep the F2.2 F2.1 start and INT.2 acceptance joins and F8.6→F2.2 unchanged.

## User Stories

1. As a designer, I want a bundled model ID in my project to resolve to the same vendor-relative filename and bytes in Dioxus as in React, so that my copied project opens on another browser.
2. As a designer working offline, I want shipped catalogue bytes available without a runtime Internet request, so that portable archives do not depend on a remote vendor site.
3. As a designer, I want model bytes transported unchanged even when the renderer cannot decode or preview them, so that archive creation does not apply preview-only validation.
4. As a designer, I want unsupported IDs or unavailable packaged files to report a precise error, so that an incomplete copy is never presented as successful.
5. As a maintainer, I want the provider to preserve saved Ergogen asset identities and existing special filename aliases, so that old projects continue to resolve the same model references.
6. As a maintainer, I want the large model catalogue served as static files rather than embedded into the page WASM, so that application delivery remains inspectable and offline-cacheable.
7. As a maintainer, I want catalogue enumeration, source-file inclusion, and emitted URL paths to agree, so that adding or removing an upstream model cannot silently break archives.

## Implementation Decisions

- Discover the same vendor `3d_models` tree and extensions that the React catalog uses: STEP, STP, WRL, and STL files. Generate a deterministic private metadata index containing the stable asset ID, reference filename, media type, original source path, and manifest-safe emitted static path. Stage each emitted file under an opaque deterministic filename with a safe extension; keep the original path and React filename solely in trusted generated metadata.
- Preserve React's exact THQWGD001 source-to-saved-name aliases: `THQWGD001-rotation.stp` → `THQWGD001 #1.stp`, `THQWGD001C-2pin.stp` → `THQWGD001C [2pin] #1.stp`, and `THQWGD001C-4pin.stp` → `THQWGD001C [4pin] #1.stp`. Asset identity/filename uses the right-hand React-compatible name; retrieval targets the sanitized left-hand source path.
- Keep URL path generation confined to the trusted generated catalogue. A caller supplies an ID, not a path or URL. Unknown IDs fail before a fetch is attempted. Encode path segments so spaces, `#`, and other URL-sensitive filename bytes identify the original static file safely.
- Make the page build serve or stage the model files at the exact generated safe static paths. Never put percent-encoded/original vendor names into the offline manifest: existing path validation intentionally rejects URL-sensitive characters. Keep files outside the Rust WASM so the normal and offline builds can load them individually and existing offline-manifest generation includes them. The build coordinator owns integrating the staging helper and source-inventory hashes into the shared M1 build/provenance script; the provider feature owns catalogue generation, static URL resolution, and the stage helper.
- Retrieve raw bytes through the deployment-prefix-aware page resource URL using the generated manifest-safe path. Do not contact vendor Internet hosts at runtime.
- Keep this capability separate from `VerifiedModelBytes` and STL/WRL/STEP parsers. Do not reject empty or content-unrecognized bytes or impose renderer-specific bounds here. Archive consumers retain Core's existing path/hash/size/entry-count validation.
- Do not invent a catalogue expected digest. The archive packer computes SHA-256 from retrieved bytes and Core verifies the `assets/<digest>` path against those bytes.
- Do not change persisted project schema, Core/worker archive transport, public generated contract types, archive semantics, or F2/F8 parent edges.

## Testing Decisions

- Test catalogue completeness and determinism against the source vendor model tree; every emitted ID/path is unique and every supported file maps to one entry.
- Test React parity for extension handling, reference filename aliases, media type, and exact ID/metadata derivation, including filenames with spaces and `#`. Cover each exact THQWGD001 source-to-saved-name mapping. Separately assert staged/manifest paths are safe, deterministic, and correspond one-to-one with generated provider metadata.
- Test exact byte equality against representative source files, including an alias case, and verify unknown IDs fail without constructing an arbitrary URL. Empty source bytes, if ever present, remain opaque at the provider boundary.
- Build and serve the Dioxus static package at root and subpath. Fetch representative model files through their safe emitted paths and compare bytes/hashes to source; verify the production output keeps them outside the page WASM and includes every provider path (not original/encoded vendor paths) in the offline manifest.
- With an installed offline worker, fetch a representative model after network access is disabled. This proves packaging/cache reachability only; it does not prove renderer compatibility or full portable archive acceptance.
- Retain existing repository checks and test the Rust page/native compilation paths affected by the private module/build metadata.

## Out of Scope

- Used-model discovery, packed-document augmentation, `embedUsedModels` input, or archive invocation. Those belong to capability ticket 01.
- The visible Project-menu action (issue 16) or F8 Export checkbox and route.
- Renderer decoding, bounds, physical alignment, model preview presentation, or widening renderer limits.
- Adding a remote fallback, changing vendor assets/attribution, recalculating source content, or changing any saved asset identity.
- Public Rust/CAD APIs, schema, generated TypeScript contracts, migration parent graphs, or RF ledger ownership.

## Further Notes

The pinned React baseline is `5a472a9426e6e38993361da402cd4ec730feb369`. Its bundled catalogue contains 88 model files (about 155.5 MB at authoring time); its generated URL strings identify statically packaged Vite assets and runtime byte reads are raw `ArrayBuffer` reads. The relevant known aliases are implementation compatibility requirements, not new identities. Current Core archive entries accept opaque zero-length payloads if their computed path/hash and existing limits are valid; the renderer path has distinct validation and must remain separate.
