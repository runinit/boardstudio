# F6C.3 linked-Core rerun on a retained Sofle-derived fixture

This is a distinct fixture run to establish repeatability after the original pinned fixture disappeared. It preserves the original fixture's missing status and does not replace its historical result.

## Fixture provenance

- Source archive: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/dioxus-34726-fixture-models-false.boardstudio`
- Source archive SHA-256: `c6ea3c0f72f999ee6736d1e65b6b2c36105b52c5a8d511285ffdc01695aa9d7e`
- Extracted project JSON: `.scratch/dioxus-keycaps-workflow/evidence/size-reflow/fixtures/soflev2-retained-34726/project.json`
- Extracted project JSON SHA-256: `981c8978101286985b2f5220e8af18a612037f5fc7cd892a52fcb881f39eae7f`
- The test creates the same-board linked-half variant in memory. It does not modify this fixture.

## Result

The existing, unchanged `replace_document_reflows_source_and_core_syncs_linked_partner_through_history` assertion passed against this extracted project JSON: **1 passed**. It verifies the accepted `ReplaceDocument` path, Core linked-partner synchronization, both-side reflow, and Undo/Redo history.

Reproduction from the worktree root:

```sh
KEYCAPS_MIRROR_PROJECT_JSON="$PWD/.scratch/dioxus-keycaps-workflow/evidence/size-reflow/fixtures/soflev2-retained-34726/project.json" \
  cargo test --manifest-path web/Cargo.toml --bin boardstudio-web \
  presentation::objects::keycap_resize::tests::replace_document_reflows_source_and_core_syncs_linked_partner_through_history \
  -- --ignored --exact --nocapture
```

The previous pinned fixture `/tmp/keycaps-fit-fixture.boardstudio` (SHA-256 `f2c38c70edbb02994c98b8fbd1eadfe99143b5d637916414dbeb4546162324bb`) and its extracted JSON remain absent. The earlier result recorded in `replace-document-mirror-core.md` remains historical evidence for that different fixture; this rerun does not restore that archive or clear its provenance gate.

This closes only the supporting Core-path repeatability check. The actual Dioxus control-to-Core browser route, React parity, overlap warning, and full F6C.3 acceptance remain open.
