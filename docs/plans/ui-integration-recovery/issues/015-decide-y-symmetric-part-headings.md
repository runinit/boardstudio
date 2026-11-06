# 015: Decide headings for other y-symmetric parts

Status: ready-for-human
Type: decision
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [012](012-review-mirrored-heading-adrs.md), [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## What to decide

Should independent parts symmetric about their local y axis, such as screens and encoders, retain the current `180 - r` rule or use plain reflection `-r`? ADR-0010's 2026-10-10 verification note raises the question. No change is implied until the user decides.

## Acceptance criteria

- [ ] User chooses the intended rule and identifies the affected part classes.
- [ ] Record rationale and any implementation or documentation follow-up separately.

## Verification

Compare representative headings at 0, 90, 180 and 270 degrees with the chosen physical orientation and current Core rule; record the user's decision and examples.
