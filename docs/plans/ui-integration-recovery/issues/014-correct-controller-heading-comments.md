# 014: Correct controller heading comments

Status: ready-for-agent
Type: task
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [012](012-review-mirrored-heading-adrs.md), [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## What to build

Describe controller headings as the plain reflection `-r`, consistent with the 2026-11-06 ADR-0010 amendment, without changing behavior or test logic. Preserve correct stabiliser descriptions.

## Acceptance criteria

- [ ] Correct misleading controller comments and assertion messages, including the helper documentation and mirrored layout paths.
- [ ] No behavior, signature, or test logic changes.

## Verification

Search controller heading wording, inspect the diff, run cargo check for the touched crate and focused existing mirrored-heading tests for pair, derive and layout. Record commands and results.
