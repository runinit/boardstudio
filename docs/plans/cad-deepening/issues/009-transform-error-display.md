# 009: One wording for placement-transform errors

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the error-wording decision in [006](006-review-follow-ups-spec.md).

## Acceptance criteria

- [ ] `AffineTransformError` implements `Display` in contracts, with a test per variant.
- [ ] The host's `affine_transform_message` is gone; host and runtime use `Display`.
- [ ] The CAD worker uses `Display`, prefixed with the operand side.
- [ ] Tests asserting the old wording are updated.

## Comments
