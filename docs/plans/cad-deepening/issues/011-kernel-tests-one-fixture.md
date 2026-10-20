# 011: Kernel tests share one prepared-case fixture

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the fixture decision in [006](006-review-follow-ups-spec.md).

## Acceptance criteria

- [ ] The `cache_complexity` and `profile_validation` integration tests are unit tests using the test-only fixture module.
- [ ] The integration `common` module is deleted.
- [ ] The native-test baseline lists the renamed tests; the count is unchanged.

## Comments
