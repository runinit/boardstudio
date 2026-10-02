# 06: Verify the complete M1 editor and retain a resumable handoff

**What to build:** The full isolated workflow runs on REVIUNG41 and Sofle at root and subpath with paired reference, compatibility, accessibility and resource evidence.

**Blocked by:** 03, 05

**Status:** Reviewed b974 release build and bounded browser checks pass. Final story audit found missing startup restoration and physical-instance UI; reviewed source a49bb798 repairs both and passes focused online root checks. Full rebuilt-release offline/subpath/startup/case race verification passes. All five final paired public pointer sessions pass the unchanged caps. Full performance/resource acceptance remains open; actual screen-reader gate blocked.

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Run full applicable integration checks and retain exact command/source/asset provenance.
- [ ] Complete visual/responsive/keyboard/focus, relevant screen-reader, raw axe/contrast and existing performance/resource comparisons.
- [x] Independent Standards and Spec reviews have no blocking findings on the final integrated candidate. Final story-completion source was reviewed at e1e8606/b50ddbdd and is source-equivalent to the a49 release; exact-source checks retain the equivalence.
- [x] Update canonical task/run/architecture/use instructions with acceptance limitations; no production cutover or React retirement.

See [current acceptance evidence](../ACCEPTANCE.md#current-release--a49bb798) for artifact scope and remaining gates. Checked items record bounded completed checks; ticket closure follows the task graph and full acceptance requirements.
