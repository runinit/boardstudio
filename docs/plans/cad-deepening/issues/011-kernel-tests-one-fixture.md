# 011: Kernel tests share one prepared-case fixture

Status: resolved
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the fixture decision in [006](006-review-follow-ups-spec.md).

## Acceptance criteria

- [x] The `cache_complexity` and `profile_validation` integration tests are unit tests using the test-only fixture module.
- [x] The integration `common` module is deleted.
- [x] The native-test baseline is unchanged: kernel tests are not in it (`scripts/check-native-tests-baseline.json` has no `boardstudio_cad` names); the count is unchanged.

## Comments

Both tests moved to `cad/monstertruck/src/tests.rs`, built through `test_support::build_profile` (a local `plate` helper there wraps it; `base_z: 0.` equals the old `z: None`). Inputs and assertions are unchanged. `tests/common/mod.rs`, `tests/cache_complexity.rs` and `tests/profile_validation.rs` are deleted. No `[[test]]` entries or scripts referenced them. Counts: unit 288 passed, 1 ignored before and 290 after; the two integration binaries (1 + 1) are gone; total 371 before and after. Clippy `--all-targets` is clean.
