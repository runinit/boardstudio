# Standards review: physical-instance session regression test

- Base: `8b00777fd499aeafe660ad0cb7f11eef97d7bbb3`
- Candidate: `a350f467b49f1f1db0d8dc4954f6a8f1e23fab0e`
- Reviewed path: `application/tests/durable_session.rs`
- Exact test diff SHA-256 (`git diff --binary BASE...CANDIDATE -- application/tests/durable_session.rs`): `b6d61f636996577dcc5b0a81abbedfaca0db72197dfecfa09cc33233e3583f88`

## Findings

No findings in the test source.

The regression drives the public `Session` events/completions and real `CoreEngine::handle` path to create an accepted document, then asserts canonical-to-physical-to-canonical scope changes, generation/export cancellation effects, and rejection of unknown board or instance IDs without changing the accepted scope. It uses the existing test helpers for extracting public effects; it does not mirror private session logic or add production APIs. The single-board instances isolate the instance identity transition from board navigation, which is appropriate for this case.

This was a source-only Standards review. The provider reported formatting/build/test execution was held until the timing window; this review does not claim test success. Production review remains recorded separately in `standards-active-project-instance-8b00777.md`.
