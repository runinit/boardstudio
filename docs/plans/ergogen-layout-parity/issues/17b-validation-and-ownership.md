# 17b: Validation and Layout ownership

Status: blocked
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Validate the grouped records and enforce Layout ownership, without silently accepting unresolved anchors.

## Acceptance

- Apply finite/default/list-bound checks, sparse defaults, negative/zero step policy, growth/truncation and nonempty logical Layout ownership. Preserve unrelated data.
- No silent acceptance of an unresolved anchor as world origin.

## Checks

Source anchors: `core/src/validate.rs`, `model.rs`, `matrix.rs`. Use focused public Core tests (document-integrity module) and `check.py quick --files`.

## Progress

- [ ] 
