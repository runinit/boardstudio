# Case-first private viewer compile patch

Patch: `/tmp/frontend-run/viewer-case-first-compile.patch`

The current binary mount is the Case consumer and supplies only a `JsValue` full scene with no model sources. The patch narrows private `RendererSceneProjection` and `RendererPageHost` inputs to that actual path: full `setScene` only, no unused STL/WRL variants, no unconstructed prepare/prepared/patch enum variants, and no second cached module import. It retains the checked acceptance boolean, strictly increasing page sequence, actual baseline host lifecycle, display/pick/point/camera methods, and board/top-level revision stamping.

The private host snapshot omits only the baseline public `RendererHost::update_scene` method because the page wrapper submits through its checked sequence-aware path and the copied method is otherwise dead in the page binary. `web/src/renderer_host.rs` remains byte-exact and unchanged. `renderer_host_source_sync.rs` proves the page snapshot is exactly the library source after (1) converting its first three crate-doc `//!` prefixes to ordinary comments so `include!` is valid and (2) removing that one exact method block. The test asserts the method occurs exactly once before removal and compares the entire resulting source byte-for-byte.

This is a compile-narrowing step, not a waiver of the broader F7.3 capability gates. Later consumers still need reviewed, actually constructed inputs and acceptance evidence for `prepareScene`, `setPreparedScene`, `setPreparedScenePatch`, STL/WRL decoding/model delivery and retry. Those future capability sources remain in the author worker history. Do not claim prepared-scene, patch, model, or five-consumer parity from the Case-first path.

Only four integration-owned files are included: `web/src/renderer_host_page_extensions.rs`, `web/src/presentation/shared_viewer.rs`, `web/src/renderer_host_page_base.rs`, and `web/src/renderer_host_source_sync.rs`. No integration files were edited and no Cargo/build command was run by the author.
