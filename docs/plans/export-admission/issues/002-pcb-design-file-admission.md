# 002: PCB and design-file export admission

Status: ready-for-agent
Type: build
Blocked by: [001: ZMK export admission](001-zmk-export-admission.md)
Related: [Candidate 02 research](../../../research/2026-10-20-export-readiness-dispatch-deepening.md)

## Problem Statement

The KiCad, SVG/DXF and footprint-library rows have different displayed blockers and dispatch requirements. A row can be conservative for one output yet omit another output's current prerequisite; several retained callbacks can invoke the current board rather than the board that rendered them. Designers need honest, output-specific availability without treating a click as a guarantee that asynchronous generation or delivery will succeed.

## Solution

Apply the output-specific admission seam established by ticket 001 to ordinary and Draft KiCad, SVG, DXF and footprint-library exports. Each mounted row explains its known current blocker, and a click uses its captured accepted source and selected board before entering the real export route. Keep Runtime and Session revalidation, KiCad preparation and package validation independent of display eligibility.

## User Stories

1. As a designer, I see whether ordinary KiCad is currently eligible based on the selected board's PCB and resolved wiring, so a blocker names the missing work.
2. As a designer with wiring findings, I can export Draft KiCad with its findings report when permitted, while ordinary KiCad remains blocked until its stricter requirements are met.
3. As a designer with valid ordinary wiring, I can click the mounted KiCad row and receive the ordinary handoff for the selected board.
4. As a designer whose wiring plan changes after display, I get a revalidated result or a precise refusal rather than an artifact made from silently substituted inputs.
5. As a designer whose KiCad handoff makes an authorized accepted preparation commit, only that export's capture advances; unrelated edits do not become part of its package.
6. As a designer after a package failure, I can correct the cause and retry without an abandoned operation blocking my next attempt.
7. As a designer, I see an SVG or DXF blocker for missing selected-board contours and can download the selected format when current outline geometry admits it.
8. As a designer switching between boards, a retained outline or KiCad click cannot export a different board from the one that displayed the row.
9. As a designer, I can export an admissible footprint library even when unrelated PCB wiring or firmware setup is incomplete.
10. As a designer with missing footprint definitions, I see the specific blocker rather than a misleading Available state.
11. As a designer who changes board, editor session, accepted revision or executor after a row renders, an obsolete click produces no wrong-source artifact or false success, and the new row reflects the current owner.
12. As a designer with an incomplete split design, I can still save and edit it; only the outputs needing missing inputs are blocked.
13. As a designer awaiting a completed handoff, I receive only a still-current artifact, and a failed browser download is reported separately.

## Implementation Decisions

- Reuse ticket 001's output-specific admission seam, adding the distinct prerequisites for ordinary KiCad, Draft KiCad, selected-board outlines and footprint definitions. Do not force all outputs through one readiness bit.
- Interpret known prerequisites for both mounted row display and click admission; “Available” means current eligibility to begin, not guaranteed delivery. Recheck current source at click and retain Runtime/Session checks for later changes.
- Tie each callback to the accepted source and board that rendered it. A stale detached row dispatches nothing and reports no false success; the fresh row communicates its own availability, without requiring an error on the detached control.
- Preserve Draft KiCad's findings package and ordinary KiCad's stronger wiring requirement. Retain resolved handoff checks, output-builder validation, export-owned accepted preparation, protection rules and retries.
- Keep leases, active-work ownership, final-delivery currency and download error reporting with their existing owners. Incomplete split designs remain saveable.

## Testing Decisions

- **Primary acceptance seam: mounted Export flow.** Click actual Available ordinary KiCad, Draft KiCad, SVG, DXF and footprint rows through the real Runtime/Session path using accepted, viable fixtures and controlled browser adapters. Assert each named artifact's format/content or its precise changed-source/provider failure; a directly invoked callback or fabricated ready boolean is insufficient.
- Mount contrasting blocked states with visible output-specific reasons: PCB not ready, ordinary wiring findings while Draft remains actionable, missing selected-board contours, missing footprint definitions and multi-board selection. Unrelated valid rows remain usable.
- Retain a rendered click target while changing board, physical instance where relevant, session (including same ID/revision reopen), accepted source or executor. Assert no wrong-source dispatch, stale result or false success and verify the new row. Do not demand messaging on a detached row.
- Test plan refresh during click, Draft findings contents, ordinary wiring rejection, export-owned capture advancement, unrelated-edit refusal and package-failure retry through mounted behavior. Native Runtime/Session tests may cover controlled races more precisely but do not replace mounted clicks.
- Extend existing mounted KiCad-row, Runtime export, PCB-handoff retry, Session ownership and output-builder regressions. Run changed browser and native targets and record commands/results at closure. Good tests assert external artifacts, blockers and ownership, not private row calculations.

## Out of Scope

- ZMK firmware admission (ticket 001), keycap/Case/mechanical packages (ticket 003), portable project export and Export page styling.
- Replacing Session admission, Runtime leases or final delivery with the displayed state; guaranteeing success after source changes or provider failure.
- Changing incomplete-project save policy or redesigning KiCad output contents beyond preserving the existing Draft/ordinary distinction.

## Further Notes

- [Candidate 02 research](../../../research/2026-10-20-export-readiness-dispatch-deepening.md) identifies possible KiCad/outline mismatches that still require realistic mounted evidence; they are not all proven production bugs. [Export availability](../../../../GLOSSARY.md#mechanical-relationships) names eligibility to begin, not a promise of delivery.
- Respect [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md), [ADR-0014](../../../adr/0014-document-authority-and-captured-lifetimes.md), [ADR-0016](../../../adr/0016-export-ownership-is-separate-from-preview-currency.md), [ADR-0021](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md) and [ADR-0027](../../../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md).
