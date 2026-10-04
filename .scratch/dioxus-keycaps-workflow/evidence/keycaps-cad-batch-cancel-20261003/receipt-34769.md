# Keycap preview and in-flight unmount — candidate 34769

## Candidate and fixture

- Candidate: `http://127.0.0.1:34769/boardstudio/`
- Frozen source: `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`
- Provenance SHA-256: `38e1980f3c38f31af1d66676a56d2d6cf57c7a3e4130dea6be2d1cb0eeb2f680`
- Packaging proof: `34769/package-proof.json` (root-retained; full22 command lineage, 1,373 page sources and 154 assets per route, zero route mismatches)
- Browser: named `agent-browser` session `keycaps_bnd1_preview-2318ea1a1376`, 1280×577.
- Fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Existing successful 24-cap/25-body cap-plus-legend and hide/show proof is reused from [34757 preview receipt](../../../dioxus-frontend-v1/evidence/keycaps-3d-preview-20261003/RECEIPT.md); existing STEP proof/review is reused without re-export.

## Changed journey

On the imported c6 fixture, the Keycaps 3D assembly view rendered the 24 generated caps and reported “3D preview updated.” The browser loaded the packaged `assets/cad-worker/entry.js`. I then returned to 2D, started 3D again, waited until the UI showed “Generating keycap CAD…”, and switched back to 2D while that pending status was visible. After one second, the UI remained in the physical 2D Keycaps workspace with no stale 3D completion or pending-status message. A subsequent 3D selection successfully rendered again and returned “3D preview updated.” Console and page-error logs were empty after the changed route.

The captures are `34769-inflight-unmount.png` (2D state after the pending preview was unmounted) and `34769-retry-preview.png` (successful 3D retry).

## Boundary and limits

The pending-preview unmount uses the existing Runtime disposal path, which terminates the feature-owned worker immediately and suppresses late publication; the observed successful retry confirms that the next request remains usable. This journey does not show the worker's explicit `kind: cancel` message being sent. `Runtime::cancel_keycaps_cad_preview` currently closes the worker, so that message branch is unreachable from the Keycaps UI. The new worker adapter nevertheless matches the pinned TypeScript groups-of-eight calls and yields between batches during ordinary processing. No user benefit has yet justified changing Runtime from immediate termination to a second cancellation/settlement lifecycle.

A running `build_keycaps` batch and the STEP call remain synchronous and cannot be interrupted mid-kernel. The 2D unmount result proves no stale result was published through the product path; it does not prove cooperative cancellation inside a batch or complete BND.1/F6C.5 acceptance. Existing source/revision guards, stable cap/legend identities, layer behavior, and STEP receipt are reused. No additional field, STEP, or lifecycle matrix was run.

## Parent reconciliation and RF

F6C.4 is accepted by the coordinator from source equivalence, existing Core tests and guards, plus the retained representative paired finding-navigation route. Its detailed evidence limits remain described in the [canonical criteria map](../keycaps-closure-reconciliation-20261003/criteria.md). F6C.2 and INT.2 remain accepted. BND.1 and F6C.5 remain open with their true joins unchanged.

RF-003/RF-010 handoff: the private worker now follows reference batch size/yield policy, but the product cancellation owner is immediate worker termination; the explicit worker-message branch is not reachable from current Keycaps Runtime. Retain that narrow boundary note and do not invent a Runtime redesign without a measured benefit. No task graph/status is changed here.

Capture SHA-256: `34769-inflight-unmount.png` `c076a01127645943705280de3ffca0f8fac41be88fd3ae3c17c14703e4a6dd7b`; `34769-retry-preview.png` `ec3dbe8a7cbf957d9fa7a1abe1569fbcb987926dc67afe20bcab06737bf1e7a3`.

## Bounded stale STEP attempt on candidate 34782

On 2026-10-03, I imported the same retained C6 fixture (SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`) into `http://127.0.0.1:34782/` in named browser session `f8c3-stale-step-7dd31abc1fc4`, opened Keycaps, and clicked the public **Export keycap STEP** control once. The click produced `/home/chris/Downloads/Sofle v2-keycaps (1).step` (6,246,504 bytes; SHA-256 `8ba23e9753f07e3a74e869a5f7eedbdd67b7d7b8500f4b76cf10e179f4ce52e0`). The immediate post-click public snapshot showed the ordinary enabled export control, with no generating/pending state. Since the request was not observably in flight, I did not change the selected board/revision or claim stale-delivery suppression/retry evidence. The session was closed. This attempt adds no C03 stale-delivery coverage; it confirms only that the public export settled too quickly for this bounded race check to be observed.

## Bounded active-preview revision change on candidate 34785

On 2026-10-03, I opened the retained C6 fixture in named sessions `f6c5-revision-7dd31abc1fc4` (candidate `http://127.0.0.1:34785/`) and `f6c5-ts-reference-7dd31abc1fc4` (pinned TypeScript `http://127.0.0.1:5175/`). Both reached Keycaps. The TypeScript reference showed `Preparing 3D geometry…` after selecting 3D assembly; no STEP export was repeated.

On Dioxus, with 3D assembly active, changing the public **Keycap profile for thumbs** control produced the visible `Generating keycap CAD…` status. While that status was observed, I changed the public **Keycap clearance** number input from `0.5` to `0.8` and pressed Enter. The control later showed `0.8`, the thumbs profile showed `OEM`, and the page showed both `Keycap settings saved.` and `3D preview updated.`. The latest visible request therefore recovered successfully using the changed document settings. Earlier bounded cycles also reached the same pending status after profile changes; those additional mutations are not treated as extra lifecycle cases.

The public journey did not expose a per-request generation/revision identifier or an explicit stale-CAD-result notice, so the displayed completion cannot by itself identify which worker response settled. It establishes that a real document edit was issued while preview generation was visibly pending and that the current settings subsequently completed. The production consumer in `web/src/presentation/layout_viewer.rs` cancels the prior worker when the preview input changes, clears the prior preview, and checks both its local sequence and the Runtime request result before publishing. `Runtime::request_keycaps_cad_preview` in `web/src/runtime.rs` additionally checks the captured accepted token/revision after worker completion. This source review found no gap to fix; the paired public timing plus guards does not claim direct observation of a discarded worker payload. No application source was changed.

## Mounted provider failure, retry, and delayed old-owner reply

On 2026-10-03, a focused mounted WASM regression exercised the production `LayoutCanonicalViewer` publication effect with a private controlled provider context. For accepted input `(left, token 7, revision 11)`, the provider returned `injected Keycaps CAD provider failure`; the mounted UI displayed `Keycap preview failed: injected Keycaps CAD provider failure` and kept **Retry keycaps** available. Clicking that action issued the same accepted input again. While that retry reply remained held, the test advanced to `(right, token 8, revision 12)`, completed the newer request, then delivered the old revision-11 failure. The old response did not restore an alert or appear in the mounted DOM.

Focused command (1 passed; 194 filtered): `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web --features page presentation::layout_viewer::mounted_keycaps_preview_tests::mounted_preview_failure_retries_and_ignores_a_late_old_owner_reply`.

The private test provider replaces only the request future at the mounted hook seam; the production component props, worker wire and runtime behavior are unchanged. The regression exercises visible provider failure, same-action recovery, and hook-level late-publication suppression across board and revision. It does not run the concrete `CadWorker` event/reply decoder or `Runtime::request_keycaps_cad_preview` with a delayed real Worker reply: Runtime creates the browser Worker directly and its post-completion accepted-owner guard remains source-reviewed at `web/src/runtime.rs` (generation/worker identity and current token/revision checks). No application build, package or commit was made beyond compilation required by the focused WASM test.
