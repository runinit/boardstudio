# Layout component controls, canvas context and view-group placement source review

Reviewer: Sol 6.1 High. **Standards CLEAR; Spec CLEAR** for the bounded source delta at `4c2809a604820aab7c79bf2508739ce01bd3cf9f`, with final evidence-only commit `5c983bf96d8752a445139d6dd8643cfc6c8607d3`. Base: `7d09d0a60fbb2cc12e259541614b9483ee618d29`. Worktree: `/home/chris/.local/share/boardstudio/worktrees/f73b-layout-layer-controls-20261002`. No app source edits were made by this reviewer.

This implements the independently cleared a125bbaf planning packet. It does not close F7.3b, F3.6, F7.1, F7.8 or any parent/public qualification join.

| Frozen artifact | SHA-256 |
| --- | --- |
| `web/src/presentation/shared_viewer.rs` | `b4d80211b58ceced7797ede0aceae03ee624e740fa36c6c4a0c580e89d54bf5a` |
| `web/assets/m1.css` | `a12b5b01d1971d058a78ca62ad8110ccaf702ae36067c59594051cddc6b5c6ed` |
| Implementation handoff at final docs tip | `02a08adfc7eb95741ac2dd7d63c17d9e0ab02cdb020bc1fd101e69e77cb0dfe2` |

Spec: the shared viewer now selects component controls from the active Layout preview's accepted PcbModel list when its source lease matches. It preserves exact source order, model IDs, references and filename labels through the existing `physical_component_layers` mapper. Layout source with an invalid lease does not fall back to physical Case rows. Non-Layout consumers retain their matched physical preview. The already-qualified delivery rows provide available/pending/unavailable state by exact model ID; no second model provider, asset alias or geometry path is introduced. Existing Layout projection uses the same preview model list and delivered IDs, so controls now correspond to the rendered source instead of remaining empty because no Native preview was present.

I traced the production Layout consumer, accepted preview/lease, matching delivery rows, renderer projection and display callback. Source scope/token and pointer/lease guards remain in place. Display changes still require current live viewer identity, current source and current Runtime scope before reaching the workflow-owned display state. Existing exact component IDs and global Models display ownership are unchanged. The Layout canvas name now identifies the Layout PCB assembly and mapped Layout part; Case retains its prior canvas wording. This is the contracted canvas-label correction, not a full accessibility audit of every shared control.

The CSS matches the pinned React source policy: default top 68/left 12; top 12/right 12 when the canvas inline-size container reaches 760 px; viewport ≤900 overrides to top 60/left 8 with 44 px button targets. The existing workspace content is an inline-size container. This preserves the important 1280 px distinction between open rails (narrow canvas) and closed rails (wide canvas); it does not substitute a viewport-only breakpoint. Actual integrated geometry remains public verification work.

Standards: the patch keeps selection, display, model delivery and renderer lifetime authorities intact and adds only private source selection and canvas-context helpers. It uses the existing accepted model type and existing layer mapper. No public API, saved schema, history owner or renderer contract changes occur. RF-015 accurately records the confirmed empty-control/source-selection mismatch and defers a typed source descriptor to later refactoring. The RF register parses, and source formatting and whitespace checks pass.

Verification: I verified the exact source diff and hashes, production call path, unchanged currentness/display guards, pinned React CSS and final evidence-only delta. I independently reran `cargo fmt --manifest-path web/Cargo.toml -- --check` and the frozen app-source range `git diff --check`; both pass. I reused sufficient retained focused evidence instead of launching duplicate Chrome/compiler work under host pressure:

- Source-selection Chrome/WASM test: 1 passed; log SHA-256 `200ecec2e06f6a954b0d51e25398da407bd7338729076114612b00af9cf371fb`.
- Disposable old-source-policy mutant: expected behavior failure returning `case-switch` instead of `layout-switch`; log SHA-256 `4ba7eb0405e655d81f372710a8b2ca167b6194dffbf2a38d1195dd16018ce11f`.
- Accessible-name Chrome/WASM test: 1 passed; log SHA-256 `8dcf3d4e6a07756d8a08f6ec50829a34c3e8e2e82471c1a3654e77b0942d0034`.
- Final restored-source strict page WASM all-target Clippy and page-bin check are author-reported passing in the frozen handoff. The unchanged mounted layer-menu test also passed according to author runner output, without a separately retained raw log.

The two new focused tests are pure functions hosted in Chrome, not a mounted complete viewer. The mounted existing layer-menu evidence covers its established row toggle/disabled/Escape/focus behavior; neither replaces the pending packaged Layout visibility journey. Source clearance does not claim real decoder readiness counts, full generated-keycap/module/BoardReference handling, integrated per-model hide/restore, stale-owner browser interactions, Light/Dark parity or actual 1280 px open/closed rails and 375 px geometry. Those exact existing joins remain open.

Integration note: this CSS file is based on 7d09, before the independently reviewed ready-dot token repair. Apply its narrow placement hunks while preserving the newer ready-dot success-surface rule and other root changes; do not replace the whole stylesheet with the author baseline.
