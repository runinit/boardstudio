# Native Case preview publication and worker transport repair

Base: `2bf5b534ef16f71f5bf181cc36bfab9cbeae9580`.

The exact source review identified two integration failures. Publishing a successful preview invalidated the same lease retained by its accepted snapshot. The worker client also encoded `serde_json::Value` objects as JavaScript Maps, while the packaged module worker reads ordinary object properties. That generated an uncorrelated error reply which the client ignored indefinitely.

The publication transition now transfers the lease from pending to published without cancelling it. Runtime uses that transition directly. Worker requests use the installed serde-wasm-bindgen 0.6.5 JSON-compatible serializer. Missing, fractional, zero and unsafe reply IDs reject pending callers; obsolete valid IDs remain ignored so they cannot settle a newer request.

## Red before green

- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web successful_publication_keeps -- --nocapture` failed before the repair with `success must remain visible after pending ownership transfers`. The test exercises the actual publication transition extracted unchanged from Runtime before fixing it.
- `node scripts/web/test-preview-generator-transport.mjs` failed before the transport repair: the real packaged module-worker call and missing reply-ID test both hit `test deadline: worker reply remained pending`; the obsolete valid reply control passed. The script builds the production worker graph, serves it from a temporary loopback server, and runs the actual Rust client in Chromium through wasm-bindgen-test. Blob module workers import that graph; the injected malformed-response worker is limited to negative transport cases.

## Passing verification

- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web case_preview::tests -- --nocapture`: 8 passed, including publication ownership, accepted-source identity, physical flipping and imported-route exclusion.
- `node scripts/web/test-preview-generator-transport.mjs`: 3 Chromium tests passed. The actual package returned a correlated result with exact worker/request/owner/batch/plan identities. Invalid reply IDs settled as errors, and obsolete valid reply IDs could not settle the current request.
- `node --test scripts/web/preview-generator-worker.test.mjs`: all 6 existing package/validation tests passed.
- Targeted rustfmt, Node syntax and Git whitespace checks passed. The wasm-pack run compiled the production page and test binaries. Existing unused snapshot-field warnings remain consistent with the missing consumer capability.

## Gates remain open

This is a bounded repair, not Issue07/08 or full Case acceptance. The producer still lacks registered provider/model-batch state and decoded-model publication; Issue12's model capability start gate remains blocked. Full Core-to-worker-to-Core, real model delivery/error/retry, public-root/subpath/offline and paired pre-CAD journeys still need acceptance evidence.

RF-003/RF-006 carry forward: cross-boundary tests must exercise the real transport and publication owner transition. No new refactoring takeaway observed beyond those existing records.

## Follow-up: pending and mounted lifetime cancellation

The root explicitly extended this repair to the remaining pending-only lifecycle finding. Runtime's pending, published, error and generation fields now share the private `NativePreviewState` owner. Its staleness check includes pending and error owners; cancellation retires all leases and advances generation. Publication requires the exact active pending lease and generation, so an old completion cannot overwrite a replacement owner. Source identity checks remain in Runtime and still include accepted scope/token/revision/scene pointer and Core executor identity.

`CasePanel` uses the production `use_native_case_preview` hook before its optional accepted-source return. Source changes and unmount cancel Runtime ownership. A per-source lifetime flag also prevents queued detached tasks from acquiring an owner after their source was replaced or unmounted. This flag is separate from the producer's lease, which remains the authority for already-started work.

Before the fix, the pending scope-away/back regression failed with `scope loss must retire a pending-only owner`. Mounted production-hook regressions failed because a queued task still acquired an owner after unmount and a pending task still published after unmount. They use a detached executor, rather than Dioxus-owned task cancellation, matching the browser scheduling boundary.

After the fix, all 10 native Case preview ownership/capture tests and all 3 mounted lifecycle tests pass. The third mounted test covers leaving and returning to the same source before a detached start polls; only the replacement request starts. Native cancellation also rejects the old completion without invalidating its replacement. The 3 Chromium transport tests pass again after the Runtime/hook integration build. These focused seams do not replace the final public paired journey or model-batch acceptance evidence.
