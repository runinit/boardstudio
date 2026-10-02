# Shared viewer model-delivery planning review

**Decision: revise before publication.** Reviewed draft `/tmp/frontend-run/shared-viewer-model-delivery-contract.md`, SHA-256 `5e9b7172dbec688614262f7db08d2c2995b073908d334408e673b3cb9ecea390`, against integration HEAD `3c83cc0f0d151f60518ae5ecaa5ed3e2dfef281e`. Source/read-only artifact review only; no Cargo/browser/source edits.

The main seam is sound: existing Artifact preview operations, verified SHA asset storage, existing ReadStep and renderer STL/WRL decoders suffice. `PcbModel.id` joins decoded meshes to transforms; renderer object/pick identity is `PcbModel.reference`, resolved by the current workflow owner through current-board part membership. No fake Part ID or new public renderer/Core API is needed. Full current owner/projection generation guards and same initialized module reuse are required. Canonical F7.3 start F7.1 and acceptance INT.2+BND.1 are correctly preserved; all five consumers and F7.8 remain open.

I independently read the retained archive: SHA `833533fa7fee58180c4721be54325c65b8600a935f75ac44a95c9de9059ab93e`, 89 parts, exactly four listed STEP/STP assets and no BoardReference. Public evidence supports SW8→main-right-keys-SW8 selection. The 124-instance count is a retained source/QA expectation, not a new live measurement by this review. This fixture necessarily exercises native Prepare/FinishPreview, not imported-board PreviewBoard.

## Required corrections

1. **Name the concrete native-preview generation bridge.** `app/src/export.worker.ts:66–132` invokes `@boardstudio/v2-kicad::exportErgogenForms`, a reserved-net allocator, source serialization, unresolved model-path rewriting, and exact plan token/revision/job IDs. `kicad/src/ergogen.ts:52` also performs model-path rewriting and legacy-arc upgrading. Current `scripts/web/build-layout-generators.mjs` packages only Ergogen index/catalogue/render, not this KiCad wrapper or worker runner. Existing artifact operations are callable, but the complete middle step is not already packaged. Identify the smallest private packaging/adapter ownership and preserve those behaviors; do not substitute raw render or invent a new public API. Keep source paths in the contract/evidence rather than publication issue implementation instructions.

2. **Retain BoardReference scene transform explicitly.** The plan mentions PcbModel transforms but omits supplying the renderer's `reference` field. React `AssemblyScene.tsx:145–160` supplies it; `renderer/src/wasm.rs:1302–1323` multiplies reference pose/elevation into board/component transforms. Carry the enabled reference with the preview; respect effective physical projection's reference-disable rule for flipped instances. Add a nonidentity pose/elevation reference case; the four-asset fixture alone cannot establish this.

3. **Make decoder integration evidence decisive.** F7.3a currently permits existing STL/WRL decoder tests instead of exercising the newly introduced byte-provider/private module bridge. Existing decoder tests do not prove the adapter calls those exports, attaches meshes under correct IDs, or handles scope changes. Require a genuine STL and WRL delivery case through the actual adapter/render path (paired public fixture where practical); retain existing decoder tests as supporting evidence. Keep STEP/STP archive/reopen, missing/error/retry and late-result controls.

4. **Split the three canonical consumers into independent vertical tickets.** Replace F7.3b's Layout+Keymap+Keycaps grouping with one child per consumer, each owning its private canonical projection and workflow selection mapping and demonstrable 2D/3D route. Keep Parts sample as its own child. The common model bridge belongs to the Case delivery owner first; subsequent children reuse its reviewed private contract. Shared registration/mount/CSS edits belong to root and land serially, while isolated projection work may proceed in parallel. Record this as implementation coordination, not invented canonical graph edges or INT.2/BND.1 start blockers. Relate the Case child to existing issue02 as a bounded continuation so it does not duplicate or falsely close that ticket.

In the consumer acceptance details retain the source generated-keycap model suppression (`assemblyPreview.ts:242`) when generated keycaps are present, and preserve the exact separate module/keycap producer identities. No full parent closure or new architecture finding is justified by these drafts.

## Corrected contract and separated children — final text review

**Decision: clear for planning publication**, subject to the separately required Standards review and ordinary root publication process. Reviewed corrected contract SHA-256 `e3529ac79f1b7d69211a8fa4cdc9e4d1170ec39a3c6510ab458dddc3a634b946` and these exact draft hashes:

- Case continuation 02: `a9c149b9c2f9413b3c86badabfb00b7a9a319088d565fb054cd2bbe97e44abca`
- Layout 03: `eed6600b24d320d7332212ba34288e15c65d5239e66225735e5bcf3165ce991c`
- Keymap 04: `889615614ed5a3337ae6f21c67371b41f15b53d00892bdcb26737d2b3a79e6e1`
- Keycaps 05: `8a95457c105fc9f1fcaa03aaf41f7a77709314daedf332d7f0000e0c295d491a`
- Parts sample 06: `294179f8eda0d7af800f2f819d7372446dfbec76ff90be5b497f22a5a27c5f45`

All four requested corrections are present. The native preview bridge now names existing KiCad generation, reserved-net/source/path/arc handling and exact plan correlation, and acknowledges absent current packaging. BoardReference pose/elevation and the flipped-instance disable rule are explicit with nonidentity-reference coverage. Both STL and WRL must pass through the actual provider/decode/scene adapter; decoder-only evidence is insufficient. Layout, Keymap and Keycaps now have separate consumer slices, with Parts sample separate and explicit independent projection-generation identity. Source selection/edit ownership and generated-keycap suppression remain intact.

F7.3 still starts after F7.1 and retains INT.2+BND.1 as acceptance joins; the later F7.8 join and canonical 62-parent graph are untouched. Common-bridge-first landing and serial root registration/mount/CSS are implementation coordination, with isolated consumer projection ownership, not new canonical prerequisites. Draft02 explicitly continues the already-published Case issue and must be published as its continuation/update rather than counted as a duplicate additional child. The four new consumers do not claim adoption or parent acceptance merely by publication.

No remaining material Spec mismatch in these bounded text changes. This clearance does not approve an implementation, public API widening, or geometry/format changes and does not establish compiler, browser, model-delivery, accessibility, or lifecycle acceptance. No source/build work performed.
