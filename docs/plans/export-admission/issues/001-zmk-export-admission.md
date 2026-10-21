# 001: ZMK export admission

Status: ready-for-agent
Type: build
Related: [Candidate 02 research](../../../research/2026-10-20-export-readiness-dispatch-deepening.md)

## Problem Statement

The ZMK Export row can say “Available” when its plan has a controller ID and assignments but lacks a reviewed controller profile or other firmware prerequisites. A click may then fail later in the firmware request. A designer needs a useful current blocker before starting, without treating availability as a guarantee of delivery or changing the export's captured ownership.

## Solution

Make the ZMK row's displayed availability and click admission interpret the same currently knowable firmware prerequisites for the accepted board and physical instance. A ready click enters the real export route; a changed source or unavailable requirement produces a precise refusal. This first vertical slice establishes the output-specific admission seam for later output families, without moving Session ownership, export leases, output-builder validation or final delivery.

## User Stories

1. As a designer, I see ZMK “Available” only when the current accepted board's known firmware prerequisites admit an export attempt, so the row does not overpromise a handoff.
2. As a designer missing a reviewed controller profile, assignments, supported split roles, transport or required peripheral setup, I see the specific blocker and cannot start that export from the disabled row.
3. As a designer with a complete current plan, I click the mounted ZMK row and receive the firmware handoff, or a precise refusal if an asynchronous prerequisite changes or generation fails.
4. As a designer who switches board, physical instance, editor session or accepted revision after the row renders, an obsolete click cannot dispatch against the new owner or produce a wrong-source package; the fresh row represents the new owner.
5. As a designer whose firmware plan or executor changes during preparation, I do not receive stale output or misleading success.
6. As a designer with an incomplete split, I can still save and reopen the project; ZMK explains the missing prerequisite while unrelated valid exports remain available.
7. As a designer, I can retry after correcting a blocker or a failed handoff, and the current row reflects the repaired accepted design.
8. As a designer starting overlapping firmware exports, I receive both packages if both captures remain current, while only the newest relevant operation updates status.
9. As a designer awaiting a finished export, I still receive its artifact after active work settles if its capture remains current; a browser download failure is reported separately.

## Implementation Decisions

- Establish an output-specific admission seam for ZMK using its accepted board, physical instance, current plan and reviewed hardware prerequisites. Use the same interpretation for displayed availability and click admission, while retaining the firmware builder's independent validation. Do not add a universal readiness bit or forwarding façade.
- Treat “Available” as current eligibility to begin, not guaranteed delivery. Re-evaluate the captured source and plan on click and at existing Runtime/Session checkpoints; strict export captures do not rebase like field edits.
- A stale retained callback dispatches nothing and reports no false success. If its row has detached, the new row shows the current owner's availability; do not require an error message on the detached control.
- Keep Runtime's independent source/worker revalidation, Session's operation admission, export leases, in-progress checks, and final-delivery checks separate. Final delivery evaluates the retained capture without requiring the operation to remain active; disposable preview scheduling does not itself invalidate an export.
- Apply hardware completeness only to outputs that depend on it. An incomplete split design remains a valid accepted/savable document, not a global error or an invented default configuration.

## Testing Decisions

- **Primary acceptance seam: mounted Export flow.** Mount the real Export page with Runtime/Session and controlled browser adapters. With a genuinely reviewed controller profile, assignments and applicable split hardware, click the rendered Available ZMK control and inspect the actual firmware handoff. A fixture-provided ready boolean or direct Runtime call is not a substitute.
- For missing profile, assignments, split roles, transport and applicable peripherals, check the mounted row's disabled state and specific visible blocker; confirm incomplete projects remain saveable and unrelated outputs still work. Where the provider cannot generate in the test environment, assert its exact reported limitation rather than a pretend download.
- Retain a rendered click target across board/instance change, same-ID/revision reopen and executor replacement; assert no wrong-source dispatch, artifact or false success. A freshly rendered row must reflect the new owner; do not require messaging on a detached old row.
- Test plan/source changes between display and click, failure then retry, and genuine firmware-request rejection. A row that was Available may fail later, but must explain the failure rather than silently succeed or dispatch a different source.
- Drive controlled asynchronous delays, source changes, overlapping firmware operations, successful final delivery after operation settlement and download failure. Assert both still-current artifacts can deliver, only the newest relevant status wins, stale captures deliver nothing, and download errors surface separately.
- Add focused native Runtime/Session/Core tests where faster or more precise; they supplement, **never substitute for**, mounted ZMK click evidence. Extend existing ZMK row, owner, retry, Runtime delivery and firmware-adapter tests. Run the changed web/browser and native targets and record commands/results at closure.

## Out of Scope

- Replacing Session ownership, export leases, final-delivery checks, Core output validation or browser download reporting with UI readiness.
- Making every Available output always succeed despite concurrent edits, provider failures or absent physical hardware; adding missing output providers or fabrication guarantees.
- KiCad, outlines, footprints, keycaps, Case STEP and generated mechanical admission (tickets 002–003); global completeness validation, portable project export, Export layout and the unrelated Layout architecture candidate.

## Further Notes

- [Candidate 02 research](../../../research/2026-10-20-export-readiness-dispatch-deepening.md) distinguishes demonstrated predicate gaps from candidate mismatches that require mounted and real-Session tests. Its ZMK test-plan fixture lacks a reviewed profile: evidence of a weak predicate, not proof of a production download failure. Ticket 001 establishes the seam before the other output families use it.
- [Glossary: accepted document and editor session](../../../../GLOSSARY.md#editing), [incomplete split design and handoffs](../../../../GLOSSARY.md#mechanical-relationships). Preserve [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)'s strict export captures, [ADR-0014](../../../adr/0014-document-authority-and-captured-lifetimes.md)'s authority boundary, [ADR-0016](../../../adr/0016-export-ownership-is-separate-from-preview-currency.md)'s delivery lifetime, [ADR-0021](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md)'s mounted/native test split and [ADR-0027](../../../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md)'s output-specific incompleteness policy.
