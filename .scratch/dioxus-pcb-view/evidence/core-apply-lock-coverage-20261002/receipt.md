# Core ApplyElectrical persisted-lock coverage

Source baseline `73b687ba83c6fa6db7685acb809f0b4ca31bd975`; test in `core/tests/electrical_wiring.rs::apply_request_reuses_persisted_selected_board_locks`.

The test opens a two-board Core document with a saved `row/0 = P1` lock on board A and same-key `row/0 = P2` on board B. It resolves target board A with an empty request lock map, verifies the result still assigns P1 and marks it locked, then submits the exact plan to `CoreRequest::ApplyElectrical` at the current base revision. It requires a committed Scene at one next revision, a generated row net to the controller, and both persisted board lock maps unchanged. This directly exercises the current Apply re-resolution's empty request-lock behavior against persisted selected-board locks. The expected result is green; no defect/red is claimed.

## Verification

- `apply-lock-coverage-green.log`: targeted request test, 1 passed. SHA-256 `d11ceb449219583a363f34bf4c9694e12a4080dcb5fcb5a17ceee76b401edba0`
- `electrical-wiring-suite.log`: full `core/tests/electrical_wiring.rs`, 11 passed. SHA-256 `39514df17e5e3b9212814c547a2ef982db8e71a7d59f0f7deb0f91d9d12695d3`
- `fmt.log`: Core rustfmt check passed. SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `diff-check.log`: `git diff --check` passed. SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

This is Core request-level coverage only. It does not change Core behavior or API, and does not accept F5.2/F8.2, the UI Apply journey, or any parent gate. RF-009 remains the source/behavior accounting handoff; no new RF item is proposed.
