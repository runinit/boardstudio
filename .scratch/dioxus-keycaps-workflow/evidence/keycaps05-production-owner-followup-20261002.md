# Keycaps issue05 production navigation-owner follow-up

This follow-up addresses the production-composition HOLD on `646151d5`. It preserves the earlier commits and receipts as historical evidence; this report applies to final source commit `9f192d7ccaaeffbc5c85165292c3646cafb144ec`.

## Frozen source

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002`
- Branch: `codex/keycaps-navigation-mounted-owner-20261002`
- Parent: `6a4c8bed6b45b43ee283f6d318b28d8f334ee645`
- Source commit: `9f192d7ccaaeffbc5c85165292c3646cafb144ec`
- `web/src/presentation.rs` SHA-256: `ef7d705026b28cd0b3bab009db34a2562fdf55029fb6f29f71a7c7cdc54cf0bd`
- `web/src/presentation/keycaps_fit.rs` SHA-256: `cc41fc7bc9bf79a731721ef59d857058d174fb92e2b986f3cc68bd568e268389`
- `web/src/presentation/case_viewer.rs` SHA-256: `30baed752fb68f63108a9b2d1c775126f7579013fe85b85b51ba89d4043ebe34`
- `web/src/presentation/keycaps_navigation.rs` SHA-256: `640ef013f23a78ca6cb1605c140b376f94fbeb8a6f25c2ee600e7c27e362a605`

## Production composition changes

The Editor and mounted lifecycle probe now share `use_pending_layout_fit`, which observes pending-fit and owner changes, admits only the captured live destination, settles invalid requests, derives destination geometry, dispatches camera/focus effects, and clears the pending request. The Editor continues to own Runtime, accepted model, workspace, and canvas surface. The hook receives those through private callbacks and retains no independent domain state.

The Editor now routes requests through `admit_accepted_request`, the same accepted-source seam exercised by the stale/current/removed-target regression. It validates workspace, scope and generation, accepted document/session epoch/token/revision/active board, current fit result and finding, live mechanical-layer availability, target membership, and route context before returning route effects and the delayed-effect owner. The controlled test document's revision matches the accepted fit source.

Case owner projection now reports the active Inspector destination: a selected generated/mechanical layer takes precedence over a retained saved-body selection. Navigating to a saved body clears a same-scope layer intent before setting the body. This is an intentional Dioxus current-Inspector/accessibility correction: the separate retained layer selection would otherwise keep the prior layer Inspector active, so the body target would not be the reachable Inspector owner. It is not described as literal React `showFinding` parity, which only sets the body selection. Same-scope body-to-layer replacement invalidates delayed work for the old body.

All additions remain private to the Dioxus presentation layer. No Session/Core ownership, public API, file format, or generated contract changed.

## Verification

On this exact source, the following checks passed:

- `wasm-pack test --headless --chrome . --no-default-features --features page --bin boardstudio-web -- keycaps` — 16 passed, 0 failed. This includes the mounted production pending-fit hook and the accepted-navigation admission test.
- `cargo check --target wasm32-unknown-unknown --no-default-features --features page --all-targets` — passed.
- `cargo clippy --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — passed.
- `cargo fmt -- --check` and `git diff --check` — passed.

The mounted test click calls the production `admit_accepted_request` with a controlled accepted source, fit state, document, workspace, scope, and generation; it dispatches the returned production route into the same pending-fit hook used by Editor, then exercises destination fit and queued frame focus. The mounted cases cover same-scope selection replacement before fit, same-scope replacement before animation-frame focus, current-owner focus, and unmount suppression before scoped Signal reads. The accepted-navigation test separately covers a stale fit result and removed target. Route tests cover generated Case-layer precedence and desktop Inspector pin behavior. The Case layer-over-body projection test proves that a retained body does not remain a co-owner when a generated layer is active.

## Review disposition and remaining gates

Sol's prior HOLD on commit `646151d59113fb1aed2f4f7434904bc81e3699cf` is preserved at `/home/chris/.local/share/boardstudio/reviews/keycaps05-mounted-owner-review-646151d5-sol-20261002.md` (SHA-256 `068c11acbf12b5c8ae0b4568a8567c5223cd84c7f9769e53a2e78b9b7024524c`). The reviewer requested accepted revision/epoch provenance and an aligned test fixture; both corrections are included in this frozen follow-up. No clearance is claimed until the exact-source re-review is recorded.

This is a bounded source repair, not the paired React/Dioxus public browser journey. Focused SVG finding-marker behavior, the missing original fixture identity, and issue05/parent joins remain open.
