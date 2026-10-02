# Corrected next-ticket drafts: independent final review

Reviewed all ten corrected files under `/tmp/frontend-run/next-tickets` against every finding in `next-tickets-review.md`, integration HEAD `598b2c026941130f9b95ff4fee57353f8eefcb0d`, the parent task graph/specifications and pinned React `5a472a9426e6e38993361da402cd4ec730feb369`. Relevant inspected React files remain unchanged from that pin.

**Publication verdict: ready. No material planning blockers remain. Dispatch is conditional on the explicitly required concrete contracts and existing dependency gates; this review does not approve an unspecified callable seam.**

1. All first slices now correctly describe INT.1 as the parent start edge only. Parts, PCB and Keymap each require coordinator-owned mount/input/scope/callback proof. PCB includes a real canvas/controller. No invented public facade, blanket INT.2 dependency, or dependency on whole downstream workflows remains.
2. F4.1a includes Ergogen plus imported definitions and project overrides, labels/aliases/exclusions and meaningful selected detail. Verified `scripts/web/build-layout-generators.mjs` packages the existing `ergogen/src/index.ts` catalogue export at the named generated path; its top-level wrapper exports only isErgogen/render. The draft correctly requires private adapter/provenance/typed-deserialization proof rather than claiming that wrapper exposes catalogue.
3. F4.1b includes the eight bundled assembly presets and imported module source with project precedence, grouping/variants, details, source-error/no-match and re-entry retry. No invented Retry button remains. Async ownership is High. At implementation, row selection must retain PartsLibrary's first-variant behavior, already covered by reference parity.
4. F5.1 preserves the reference shared host hiddenLayers lifetime, separate module state, dynamic face-mapped layers and standard groups. The real scene and source proof replaces the erroneous assumption that Layout graphics or PreviewBoard alone supplies PCB rendering. Pad/copper/drill interactions remain governed by the cited reference.
5. F6K.1a explicitly covers virtual Base, stable-ID fallback, legacy bindings, transparent non-base fallback, correct membership and shared selection/native filtered select. High ownership and the F3.1 acceptance join are retained.
6. F6K.1b now permits Base rename, protects only the first layer from removal, and preserves blur/limits/validation/history/storage behavior. This matches KeymapPanel and Rust RenameLayer/RemoveLayer; publication should retain this source-backed reconciliation of the parent shorthand.

Dispatch notes now name actual Rust/WASM checks and page/browser provenance. Published children inherit shared ACCEPTANCE.md, including native/repository checks where affected and carried AT/resource gates. Coordinator must attach exact files, callable contracts, current fixtures/build identities, owner allocations and applicable checks immediately before dispatch. Parent completion remains open.

Refactoring: no new takeaway observed; the source-accounting and ownership observations reinforce RF-009, RF-001/RF-002 and RF-005/RF-006. Record the stated reconciliation when publishing. No repository mutations, builds, browser actions or subagents were performed.

## Reviewed draft snapshots

| File | SHA-256 |
| --- | --- |
| `F4.1/F4.1a.md` | `564c48d841a05c6a12cf505aa04c3f42d4c8199ab92946018ba3928691af2360` |
| `F4.1/F4.1b.md` | `b31af289c724aa44ce9c7754229e2ec6ff01f69a3f266207b09a44720201e446` |
| `F4.1/dispatch.md` | `603c9df386f56a5f2ba3023f90a3817ab6186add9c2a47d4cb21f6b6c6aaa2d9` |
| `F5.1/F5.1a.md` | `1a2df9c6dfb6ef3db2ee6eb3391d3cbe76fd76ab4660dd932f967fcc32665333` |
| `F5.1/F5.1b.md` | `042d7d7f40b4927e0a5038ef070eea830ea73d9c3825d77620be2f042fc265c5` |
| `F5.1/dispatch.md` | `4db988a9ba184d9e3b13455b2a43e3d8e2ce851b0cdad4c5c5c89e53280a2aa2` |
| `F6K.1/F6K.1a.md` | `c0789fcd81239b792b21ca76e2b9b8563dbb4817566d0373b83fc27522fc021e` |
| `F6K.1/F6K.1b.md` | `c03b3cebf1e493e2561a58fcddeeee54036f84f2e6e2434f93805cf071f7104b` |
| `F6K.1/dispatch.md` | `d48b9b6cc22bf2bc4e71c09fc861379ce469a76882613036c7059bab294b4b70` |
| `manifest.md` | `0683d76ff22ce5e40ca66798e513431f58aafecd2e39e8264953f76113eb4ed3` |
