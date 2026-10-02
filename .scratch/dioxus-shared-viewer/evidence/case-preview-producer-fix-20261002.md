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

This is a bounded repair, not Issue07/08 or full Case acceptance. The producer still lacks registered provider/model-batch state and decoded-model publication; Issue12's model capability start gate remains blocked. Pending-only scope-away/back and unmount invalidation remain an outstanding P2 source finding. Full Core-to-worker-to-Core, real model delivery/error/retry, public-root/subpath/offline and paired pre-CAD journeys still need acceptance evidence.

RF-003/RF-006 carry forward: cross-boundary tests must exercise the real transport and publication owner transition. No new refactoring takeaway observed beyond those existing records.
