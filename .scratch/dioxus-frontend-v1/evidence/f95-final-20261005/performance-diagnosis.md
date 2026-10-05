# F9.5 pointer performance diagnosis and repair — 2026-10-05

## Required defect and existing RED

Immutable candidate `df6b04b9` at `34829`, pinned TS `5a472a9` at `5175`, exact generated archive bytes, viewport1280×720/DPR1, native pointer input,10warmups/100measured samples. Absolute30/100/200-key caps remain33/50/100ms. No relative threshold, Undo/Redo, mobile or accessibility work added.

The retained paired sessions are `paired-df6b04b9-final/`; `stopped-summary.json` distinguishes completed failures from the deliberately stopped wrapper.100-key candidate failures recur at75.09/78.03/80.11ms. Session1 also fails200keys at185.635ms. These are product-performance REDs, separate from later200-key setup failures.

| Variant | Keys/parts | Input→rAF p95 ms | Input→DOM median ms | Event timestamp→capture median ms |
|---|---:|---:|---:|---:|
| candidate | 30/90 | 28.110 | 13.880 | 5.208 |
| candidate | 100/300 | 75.090 | 51.298 | 22.683 |
| candidate | 200/600 | 185.635 | 136.962 | 64.522 |
| reference | 30/90 | 19.400 | 4.400 | 0.500 |
| reference | 100/300 | 23.200 | 9.100 | 0.500 |
| reference | 200/600 | 25.700 | 14.950 | 0.500 |

The latter dispatch delay is retained context, not added to or substituted for the existing latency endpoint. The observer still measures first rAF after the target SVG transform changes, not physical display presentation.

## Ranked falsifiable hypotheses, then one bounded profile

1. Newly added global notification/reconciliation or workspace projections repeat accepted-source work on every gesture update. Predict dominant page CPU and repeated projection/allocation rather than worker CPU.
2. Runtime model reads deep-copy accepted geometry. Predict clone-heavy stacks. **Ruled out by source:** document, accepted scene and display preview are `Arc` values.
3. Core preview/snap work dominates. Predict worker CPU/backlog. The profile shows the Core worker mainly idle, so it is not the dominant measured branch.
4. SVG layout/paint dominates. Predict layout/paint rather than page WASM projection CPU; retained capture→DOM scaling and profile instead prioritize projection work.

The single100-key profiler replay is `diagnostic-df6b04b9-100/run.json`, with `pointer-100.json` and the raw `pointer-cpu-profile.json`. Its actual public pointer loop produced100samples, p95 **77.885ms**, hence RED against50ms. `diagnose-pointer.mjs` is the narrowly adapted diagnostic copy: only100keys and CPU profiling around the unchanged native input batch. Profiling timings are diagnostic, not a replacement release measurement.

Over24.17s of sampled capture, the page callback tree accounts for21.64s; the Core worker is active4.13s and idle19.83s. About6.00s belongs to a Runtime completion→notification→App branch; the Dioxus render ancestor contains a further10.88s. These inclusive times overlap through callers and must not be summed as independent costs.

The shipped WASM names are stripped. Correlation to the immutable snapshot's existing named intermediate uses function-body similarity (not an exact symbol map): the hot leaf chain correlates to RandomState hashing/HashMap insertion, then `objects::tree::context_for_part`, `selection::eligible_live_ids`, the App subscription, `Runtime::changed` and `Runtime::complete`. The raw profile remains authoritative; the source independently confirms that exact call chain and its repeated board-set construction. No production instrumentation or extra build was used for the diagnostic.

## Concrete source cause and minimal fix

`presentation.rs`'s Runtime subscriber called `eligible_live_ids` for each notification and again for anchor validation. That function walks all live board parts; `context_for_part` constructs a board-wide `HashSet` for each part. A300-part board therefore reconstructs300boardsets per eligibility projection, even when its accepted source has not changed. The performance fixture has no matrices, but the same expensive membership algorithm was still used. Gesture samples, animation frames and Core preview completion all notify. In the passing`a49bb798` source, App's subscriber lacked this selection-adapter work; the adapter entered in`547b0762`.

The patch caches only the live/eligible membership projection in App. Its source key includes token, session epoch, active board, physical instance, and the accepted document/scene `Arc` identities. It holds both immutable sources strongly, preventing allocator address reuse. Accepted edits/deletes/imports/replacement snapshots or board/instance changes recompute; absent accepted source clears membership. Preview/camera/selected-ID notifications reuse it.

Every notification still validates context, reconciles selection, checks anchor ownership, advances the version and renders the preview. The `RefCell` borrow ends at the projection statement before nested `SelectParts` can notify. Runtime and gesture scheduling are unchanged. The second anchor lookup uses the same cache against the fresh model, so an actual owner change still misses.

## Owning regression and limits

`matrix_transform_lifecycle::selection_retention_tests::accepted_membership_reuses_preview_notifications_and_retires_on_source_change` uses real Session/Core acceptance and the exact cache called by App. An initial extraction preserved repeated computation; the test failed for the expected reason: **2projection calls versus1** after the first gesture/display/selection update. `membership-native-red.log` retains that actual failure. After the cache fix, the focused wrapper reports **1passed**, including100preview/selection/camera notifications and independent board/instance/token/epoch/document/scene/revision/close/reopen invalidation checks. `membership-native-green.log` is final. This is a projection/invalidation proof, not a browser latency GREEN.

`node --check` on the canonical driver and `git diff --check` pass. Coordinator still owns the mandatory combined native/page/reachability/affected-WASM gate, commit/package and actual final paired performance sessions. The existing Case-only presentation-root owner mapping is stale for this production change and must be replaced with reviewed selection-owner coverage plus the native cache regression. No final performance success is claimed before the new package is measured.

## Separate driver setup defect

The old candidate import branch waited only for any `.m1-canvas`. Session2's trace shows upload completing15:43:36.099Z, SVG wait passing15:43:36.194Z, then missing`key-4-19` at15:43:36.219Z; session3 similarly fails after the boolean SVG check. The imported scene was not yet the expected200-key scene. This does not invalidate the completed100-key performance failures.

The canonical driver now waits for the imported canvas's exact expected part count **and** targetpart before hit testing. Sampling, thresholds, fixture bytes and input remain unchanged. The existing initial startup observation remains; early per-size0-control snapshots remain honestly early observations. The new readiness condition executed successfully in the100-key diagnostic; final full sessions must establish all sizes after the product repair.

## Frozen files

- `web/src/presentation.rs` — `0eec0ed1720c617d543379feff15e8d2c949cf0d81ad637e1d73aed7ccc081e5`
- `web/src/matrix_transform_lifecycle.rs` — `b346b1ca4738897fe66fd92acea0533bfcf725762c5e324fe203d308f807f989`
- `.scratch/m1-production/evidence/performance/measure-candidate-pointer.mjs` — `abf17e9b80f49b10b1056fcdcad225547bbb1b6d216b871b54d9cf762bd31f2a`
