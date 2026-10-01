# 01: Reconcile the exact newer provider reference

**What to build:** Open and round-trip copied projects using the newer encoder/VIK/module providers without losing accepted migration contracts.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [ ] Record exact dev and migration source revisions and preserve original checkout/stashes.
- [ ] Resolve overlapping edits without reverting enum boxing, lossless core JSON or lifecycle fixes.
- [ ] Pass affected provider tests, contracts, formatting and strict lint or record actual blockers.
