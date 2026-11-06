# 016: Audit ADR decisions against shipped behavior

Status: claimed
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

## Work in progress (2026-11-06)

This is a partial source review, not the requested per-ADR audit. No ADRs were edited. The index has 40 records including deprecated 0029. The index's summary of 0004 is historical: its own opening pointer and 0036's amendment supersede migration with refusal of older input; `core/src/migrate.rs:10-26` confirms format 7 rejects older and newer versions. A possible index clarification is to summarize the current refusal policy; inspect the full index and related amendments before proposing exact wording.

- 0001: `core/src/outline_versions.rs:429-430` describes fixed geometry after component removal; this supports that narrow aspect, not the complete decision.
- 0002: `core/src/outline_edit.rs:116-140, 1292, 1386` identifies linked refinement and Freeze paths. Their full behavior still needs tracing.
- 0005: `application/src/session.rs:220-245` describes and implements resolution against the accepted snapshot. Queue execution and retirement need further tracing.
- 0012: `core/src/outline.rs:31, 141-143` implements the per-path 1024-point budget; grid contacts remain to be checked.
- 0026: `web/crates/runtime/src/runtime.rs:6753-6764` still rejects split firmware unless central is left and peripheral right. This is the known delivery gap already stated in the ADR, not evidence the decision should be reversed.
- 0028: `web/crates/catalogue/src/physical_setup.rs:472-507` normalizes definitions and flips instances by half; the ADR already describes this as incomplete conversion. The full conversion remains unverified here.
- 0029: deprecated feature premise remains distinct from execution safety; `core/src/script.rs:189` and its test at `:466` are starting points, not a completed safety review.

The remaining ADRs and these decisions' unchecked clauses remain unverified. No amendment is proposed as established fact; continue by reading each decision and amendment, tracing its primary behavior or relevant tests, and recording findings and limitations per record. The existing heading-only review is in [012](012-review-mirrored-heading-adrs.md). `python3 scripts/check-doc-links.py` passed before this work-in-progress note; rerun after completion. No runtime tests were run for this research.
