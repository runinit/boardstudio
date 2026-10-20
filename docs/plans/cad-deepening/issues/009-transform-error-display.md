# 009: One wording for placement-transform errors

Status: resolved
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the error-wording decision in [006](006-review-follow-ups-spec.md).

## Acceptance criteria

- [x] `AffineTransformError` implements `Display` in contracts, with a test per variant.
- [x] The host's `affine_transform_message` is gone; host and runtime use `Display`.
- [x] The CAD worker uses `Display`, prefixed with the operand side.
- [x] Tests asserting the old wording are updated.

## Comments

Wording, old to new:

- Contracts `Display` (new): "row transform contains non-finite values", "row transform is not an affine 4x4 matrix", "row transform has a singular linear part".
- Host and runtime, old: "collision operand row transform must be a finite affine matrix" (non-finite and not affine), "collision operand row transform is singular". New: the `Display` text above, one per variant.
- Worker, old: "Invalid collision query: {side}.rowTransformF64 contains non-finite values" / "must be an affine 4x4 matrix" / "has a singular or non-finite linear part". New: "Invalid collision query: {side} {Display}", for example "left row transform has a singular linear part".
- No existing test asserted the old text.
