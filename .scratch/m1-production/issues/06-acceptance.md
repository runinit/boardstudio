# 06: Verify the complete M1 editor and retain a resumable handoff

**What to build:** The full isolated workflow runs on REVIUNG41 and Sofle at root and subpath with paired reference, compatibility, accessibility and resource evidence.

**Blocked by:** 03, 05

**Status:** Reviewed b974 release build and bounded browser checks pass. Final story audit found missing startup restoration and physical-instance UI; both are under repair with public red evidence. Full performance/resource acceptance remains open; actual screen-reader gate blocked.

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Run full applicable integration checks and retain exact command/source/asset provenance.
- [ ] Complete visual/responsive/keyboard/focus, relevant screen-reader, raw axe/contrast and existing performance/resource comparisons.
- [ ] Independent Standards and Spec reviews have no blocking findings on the final integrated candidate. Earlier reviewed source remains accepted; story-completion changes require fresh review.
- [ ] Update canonical task/run/architecture/use instructions with acceptance limitations; no production cutover or React retirement.

See [current acceptance evidence](../ACCEPTANCE.md#retained-evidence--release-8f509433-and-reviewed-overlays) for artifact scope and remaining gates. Checked items record bounded completed checks; ticket closure follows the task graph and full acceptance requirements.
