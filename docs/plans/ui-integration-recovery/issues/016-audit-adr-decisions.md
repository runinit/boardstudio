# 016: Audit ADR decisions against shipped behavior

Status: ready-for-agent
Type: research
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [012](012-review-mirrored-heading-adrs.md), [ADR index](../../../adr/README.md)

## What to build

Audit the indexed ADR set against the current checkout. This is a bounded source review of each ADR's primary decision as summarized in the index, including amendments and the deprecated record's disposition; it is not exhaustive verification of every clause or runtime acceptance. For each record locate relevant implementation or tests, report evidence and classify as supported, mismatch, or unverified. Do not infer a mismatch from missing evidence. Identify concrete follow-up and propose amendments only where evidence supports them. Do not silently edit ADRs.

## Acceptance criteria

- [ ] Record each indexed ADR, its primary decision, source or test evidence, and classification, including explicit uncertainties.
- [ ] Record supported amendment proposals or follow-up investigation; distinguish historical and deprecated decisions.
- [ ] State review limits and any ADR edits made.

## Verification

Inspect the index and ADR decisions/amendments and trace representative source or tests for each. Record paths and lines and any gaps; run documentation link validation. No behavior or ADR changes are required.
