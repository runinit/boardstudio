# Spec: Remaining ticket closure (simplified)

Status: ready-for-agent
Source: Opus analysis of tickets 001, 002, 006, 008, 013
Map: [map.md](map.md). Tickets: [issues/](issues/).

## Problem Statement

Five CAD backend tickets (001, 002, 006, 008, 013) remain in `claimed` status
despite functional code being integrated and tests passing. Their formal
acceptance boxes are unchecked because they require expensive verification
(34-asset census, combined reviews, WASM/browser gates) that has not been run.
This blocks the final sign-off of the CAD backend cutover.

The root cause is overscoping: these tickets demand universal certification
(34-asset replay, combined closure review, WASM/browser rerun) for code that
is already functional and tested. The 007 ticket was already simplified using
industry-standard approaches (aggregate reports instead of per-route accounting).
The same simplification applies: reuse existing evidence, run one combined review
per track, and accept named limitations.

## Solution

Close the five tickets in two independent tracks with minimal verification:

**Track A (generated construction/export):** 013 → 006 → 008
- Close 013: record adoption recommendation and named refusals
- Sign off 006: explicit adoption decision with named limitations
- Close 008: reconcile fixture checks, reuse FreeCAD receipt

**Track B (importer/mesher quality):** 001 + 002 joint
- One combined reconciliation: explain recovered faces as diagnostics, confirm
  existing tests, bind angular measurement, run affected-shell comparison once

**One combined review per track**, not per ticket. Reuse existing evidence.
No repeated 34-asset census, FreeCAD run, or WASM/browser gate for unchanged code.

## User Stories

1. As a developer, I want 013 closed as a prototype with documented adoption
   recommendation, so that the layered construction is available for 006.
2. As a developer, I want 006 signed off with explicit named limitations, so
   that the product's adoption gaps are transparent.
3. As a developer, I want 008 closed with reconciled fixture checks, so that
   exact STEP export is certified.
4. As a developer, I want 001 and 002 reconciled jointly, so that one review
   covers both importer/mesher quality tickets.
5. As a developer, I want one combined review per track, so that verification
   is efficient.
6. As a developer, I want existing evidence reused, so that I don't rerun
   expensive checks for unchanged code.

## Implementation Decisions

- 013 is a prototype; its acceptance criteria include "named refusals" which
  means documenting what's not supported, not implementing everything.
- 006's "cone-wall crossing" and "sealed polygon openings" are named adoption
  gaps, not failures. The map already says unsupported geometry remains Unknown.
- 008's planar reader deliberately refuses curved exports. This is by design,
  not a bug. Curved features use bounded certificates.
- 001's recovered-face volume changes are mesh diagnostics (triangles now cover
  previously missing regions), not physical volume errors.
- 002's edge-free invalid-option tests already exist
  (`angular_invalid_options_refuse_edge_free_faces_in_every_lenient_route`).
- Track A and Track B are independent. 006/008 don't need 001/002's
  vendor-import qualification.
- One combined Standards+Spec review per track, not per ticket.

## Testing Decisions

- Reuse existing test results. Do not rerun for unchanged code.
- Run one affected-shell comparison (shared between 001 and 002).
- Confirm one live export-to-certificate link for 008 (countersink).
- No new tests needed; existing tests already cover the functionality.

## Out of Scope

- New geometry implementations
- General Boolean engine
- Repeated 34-asset census
- Repeated FreeCAD/external interoperability runs
- Repeated WASM/browser gates for unchanged code
- Repeated performance comparison for 013

## Further Notes

- The Opus analysis is the source for this plan's simplification strategy.
- 009 (mesh keycaps) was already signed off separately.
- 007 (vendor STEP import) remains a deferred quality follow-on. Its deep
  accounting (packages 2-6) is scoped to 014's collision certificate only.
- After this plan, all tickets in the CAD backend cutover will be resolved.