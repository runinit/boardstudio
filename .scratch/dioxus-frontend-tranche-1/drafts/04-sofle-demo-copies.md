## Problem Statement

The Project library now exposes fixture-backed cards and fresh editable copies for REVIUNG41 and Sofle v2, but it omits the pinned reference's Sofle RGB and Sofle Choc demos. Designers cannot identify or start those two distinct, measured Sofle projects from Dioxus.

## Solution

Add the existing Sofle RGB and Sofle Choc variants to the same Project library and Project menu card workflow. Their preview and summary come from the matching packaged project documents. Their project archives are prepared by the existing Sofle demo and Core services, then opened through the existing fresh-copy path so each accepted project has its own identity and normal Session, save, Undo and reopen behavior.

This refines the existing Issue 04. Sofle v2 card/copy behavior is already qualified by Issue 20; this implementation completes the two missing variants while retaining the original three-variant parent criteria.

## User Stories

1. As a keyboard designer, I want to see Sofle RGB beside Sofle v2 with its measured layout preview and board/key summary, so I can tell which keyboard I am opening.
2. As a keyboard designer, I want to see Sofle Choc beside Sofle v2 with its measured layout preview and board/key summary, so I can choose the low-profile variant accurately.
3. As a keyboard designer, I want to start either variant from both the Project library and Project menu, so the same source-backed choices are available from both entry points.
4. As a keyboard designer, I want each variant to open with its own exact measured geometry, generated definitions, two editable board halves, peripheral parts, wiring and source provenance, so I can continue the supported design rather than inspect a lookalike placeholder.
5. As a keyboard designer, I want every start to create a fresh editable project identity, so repeated RGB or Choc starts remain independently saveable and reopening one cannot overwrite another.
6. As a keyboard designer, I want browsing cards to leave the accepted project and its history unchanged, so comparing variants has no hidden mutation.
7. As a keyboard designer, I want save, reopen, edit and Undo to keep using the existing accepted Session and persistence path, so variant projects behave like any other BoardStudio project.

## Implementation Decisions

- Keep Issue 04 as the existing three-variant Sofle workflow. Reuse its accepted Sofle v2 behavior from Issue 20; implement the missing RGB and Choc variants in this bounded continuation.
- Derive each preview, summary, project document and provenance from the existing measured Sofle variant data and canonical demo construction/open path. Do not copy React UI code, recreate matrix geometry, reimplement wiring, or import upstream routed PCB artwork.
- Produce the static documents and archives with the existing fixture preparation boundary and Core archive service. Runtime opens each packaged archive via the fixture-only fresh-copy identity path already used by the two existing demos. Generic user archive identity stays unchanged.
- Preserve variant-specific MX/RGB and Choc geometry, LED/member definitions, split-board identity, peripherals, hardware/wiring, measurement provenance and required assets. Browsing remains read-only.
- Keep the Library and Project menu using the same existing card component and presentation. Project, Session and BrowserStore remain the owners of accepted state, history and persistence.
- Retain source/load/stale-owner guards. Failure injection is a separate acceptance criterion; no new provider or public API is proposed.

## Testing Decisions

- Reuse the current Sofle v2 paired card/copy evidence and existing reference Sofle Core/provider tests.
- On the pinned React and integrated Dioxus candidate, compare the three Sofle cards, exact previews/summaries, and both new start actions. Open RGB and Choc as separate accepted projects and verify variant document identity and persistence. Use one representative variant for an edit/Undo/save/reopen path and reuse Issue 20 identity and card-list behavior where unchanged.
- Use read-only project-store inspection as supporting evidence for exact IDs and variant documents; public UI actions remain the test driver. Do not add ordinary UI tests or a broad hardware/export matrix.
- The coordinator runs the affected combined compile/package check once. Retain RF-001 notes and do not claim full F2.1, all measured demos, or complete electrical/manufacturing readiness.

## Out of Scope

- Completing every demo in the measured-layout catalogue or the VIK module review demo.
- Copying original routed KiCad boards, reversible PCB construction, legacy footprints, or unverified source circuitry.
- Firmware builds, routed manufacturing validation, electrical/power qualification, component-fit qualification, or changing the bundled footprint library.
- Reworking generic archive-import identity semantics, saved-project search/listing, Project rename/duplicate, or the broader F2.1 parent.

## Further Notes

The pinned React source lists Sofle v2, Sofle RGB and Sofle Choc and builds each through the same measured Sofle project and electrical-resolution flow. The repository already retains the exact measured layouts, source hashes, variant construction and Core/provider tests. The candidate's fixture preparation currently packages Sofle v2 using that path; extending this existing boundary is the smallest complete route for the two missing variants.
