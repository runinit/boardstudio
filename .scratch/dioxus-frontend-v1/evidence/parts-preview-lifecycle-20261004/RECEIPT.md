# Parts preview currentness and nonmutation qualification

Candidate: published frontend-preset-customization-20261004, source4e716156, root34822. Reference: root5175. Desktop only; use retained saved projects, without applying generator edits.

## Public journey

Candidate Project menu shows current `Grouped journey Sofle 20261004`, `Revision 24 · Saved`. Close menu, open Parts, enter MX switch3D, change selection to Choc V1/V2 and immediately back to MX. Both frontends reset the selected sample to2D on selection changes; this is matching behavior, not failure. Return candidate MX to3D and observe `3D preview ready` under the isolated preview region. Switch2D/3D again, navigate to Layout (unmount), and return to Parts. Project menu still reports the same current name and Revision24Saved.

The paired reference follows MX3D -> Choc -> MX, repeated2D/3D and Layout/Parts. Current project remains `Sofle v2` and `Saved locally`. Its Project menu does not expose numeric revision, so no reference revision equality is claimed. Candidate public identity/revision equality and reference route/name/durability observations are separate from any whole-archive or UUID-byte equality claim.

## Executed currentness and cleanup evidence

- `frontend-preset-customization-20261004/commit-gate.log` records successful mandatory unfiltered native web lib/bin execution at4e716156. main.rs registers parts_preview and native presentation/model_delivery: the late old-generation lease test and pending-token supersession/retry tests are included. There is no separate per-test transcript; do not call the source listing alone execution evidence.
- The same gate ran the strict WASM file selector for changed presentation/parts.rs, which selects its complete test subtree, including `stale_async_result_cannot_reclaim_same_definition_after_selection_round_trip`. No Parts test is excluded. The successful gate plus inspected selector/registration establishes enclosing-run execution; no duplicate run was added merely to obtain another receipt.
- `renderer-headless.log` records a fresh exact module run: `python3 scripts/run-wasm-tests.py --files web/src/renderer_host_page.rs`, 5executed/0failed. The source is unchanged from the candidate. It tests partial-init cancellation/failure cleanup, context-loss stop/settlement/unmount disposal, repeated mounts without listener/observer accumulation, idempotent dispose/drop and submission-error settlement. These use a scripted renderer object against real DOM/listener/observer lifecycle; this is deterministic owner cleanup evidence, not a heap/GPU-memory benchmark.
- Existing shared-viewer14/14 execution covers changed-owner/unmount event rejection and source scope/token admission; see parts-preview-qualification-map-20261004.md.

Together these qualify F4.4-C04's nonmutating preview/currentness/cleanup behavior. They do not close the broader F7.3-C07 theme/DPR/consumer/stale-editor joins, or replace the missing mounted Parts renderer-initialization failure presentation check for F4.4-C03/F7.3-C10. Existing real model conversion/context-loss receipts remain reusable. No parent acceptance or mobile qualification is implied.
