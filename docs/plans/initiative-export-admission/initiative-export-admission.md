---
type: initiative
title: "Export admission"
parent: none
sources:
  - docs/plans/export-admission/map.md
  - docs/plans/export-admission/issues/001-zmk-export-admission.md
  - docs/plans/export-admission/issues/002-pcb-design-file-admission.md
  - docs/plans/export-admission/issues/003-geometry-package-admission.md
companions:
  - docs/plans/export-admission/issues/001-zmk-export-admission.md
  - docs/plans/export-admission/issues/002-pcb-design-file-admission.md
  - docs/plans/export-admission/issues/003-geometry-package-admission.md
after: []
risk: high
---

# Export admission

## Workflow

Installed BMad skills govern orchestration, ticketing, review and verification. Legacy sources and historical notes retain behavioral requirements and evidence, not their proof-map, mutation, fixed-review or ordered-closure protocols. Functional acceptance and explicit human/phase approvals remain unchanged.

## Description

Align each Export row's displayed prerequisites and click admission for the accepted design, without replacing captured export ownership or delivery checks. Migrate the three existing tickets into one epic, preserving their scope and original evidence in the frozen source tickets.

## Outcome

Designers can see why a named output is blocked, begin eligible exports from the current accepted design, and receive a still-current artifact or a precise failure without losing incomplete-project saveability.

## Requirements

R1 (001 §User Stories; §Implementation Decisions): ZMK availability and click admission agree on the accepted board and physical instance's known firmware prerequisites, including reviewed controller profile, assignments, supported split roles, transport and applicable peripherals. Demonstrate the real firmware transfer or precise blocker, asynchronous failure and corrected-plan retry.

R2 (002 §User Stories; §Implementation Decisions): Ordinary and Draft KiCad, SVG, DXF and footprint-library availability reflect their distinct selected-board PCB, resolved wiring, contours and definition prerequisites. Demonstrate ordinary transfer, Draft findings, export-owned preparation, unrelated-edit refusal and package-failure retry without blocking independent valid outputs.

R3 (003 §User Stories; §Implementation Decisions): Keycap STL, authored Case STEP and generated mechanical packages use their distinct accepted-scene, authored-case, physical-instance, mount and exact-generation prerequisites. Demonstrate each named output or precise blocker/provider limitation; preview meshes are not exact manufacturing authority and preview refresh alone does not revoke a valid captured export.

R4 (001–003 §Implementation Decisions; §Testing Decisions): A retained control cannot dispatch or deliver for another board, physical instance, editor session, accepted revision or executor. Demonstrate changes between display, click and asynchronous work, including same-ID/revision reopen, with no wrong-source artifact, stale delivery or false success; a fresh row reflects its current owner without requiring messaging on a detached row.

R5 (001–003 §User Stories; §Testing Decisions): Incomplete split designs remain saveable, reopenable and editable; only outputs depending on missing hardware are blocked. Demonstrate visible output-specific reasons and unrelated valid exports, without fabricating missing hardware or a global completeness error.

R6 (001–003 §Testing Decisions): Acceptance clicks the real mounted Export rows through Runtime/Session with viable accepted fixtures and controlled browser adapters, inspecting named artifacts and format/content or exact reported limitations. Demonstrate blocked rows and failure paths; fabricated ready booleans, direct callbacks and native-only tests do not replace mounted click evidence.

R7 (001–003 §Implementation Decisions; 001 §Testing Decisions): Captured source ownership, independent Runtime source/worker revalidation, Session admission, leases, active-work checks, output-builder validation and final-delivery checks remain separate. Demonstrate still-current delivery after operation settlement, overlapping current firmware artifacts with only the newest relevant status, stale rejection and browser download failure reported distinctly from generation failure.

R8 (map.md §Tickets; §Dependencies): Preserve exactly 001 ZMK export admission, 002 PCB and design-file export admission, and 003 Geometry-package export admission. Demonstrate 001 as the first admission-seam slice, with 002 and 003 depending on 001 and independent of each other; preserve legacy ready status as ready-for-dev plans without claiming implementation completion.

R9 (001–003 §Testing Decisions): Each ticket extends the relevant existing regressions, runs changed native and browser targets, and records commands/results at closure. Demonstrate source-specific artifacts, blockers and ownership at the mounted boundary, supplemented by focused native Runtime/Session/Core/provider evidence.

## Constraints

- “Available” means current eligibility to begin, not guaranteed generation or delivery. Share output-specific prerequisite interpretation; do not add a universal readiness bit or forwarding façade. (001–003 §Implementation Decisions)
- Preserve strict captured revisions rather than field-edit rebasing, accepted document/Session authority, export-owned preparation advancement only, and independent validation/delivery owners. (001–003 §Implementation Decisions; ADR-0005, ADR-0014, ADR-0016)
- Keep authored STEP independent of disposable preview currency; preserve exact-generation diagnostics and provider limitations rather than substituting meshes or promising fabrication readiness. (003 §Implementation Decisions; §Out of Scope; ADR-0015)
- Source research distinguishes a demonstrated weak ZMK predicate from candidate mismatches; it does not establish every candidate as a production failure. (001–003 §Further Notes)
- The approved migration creates one epic and only the three legacy identities; it authorizes documentation and active-initiative selection, not implementation. (2026-11-15 user decision)

## Done when

- All named output families demonstrate honest mounted availability, specific blockers and real click admission using accepted fixtures through Runtime/Session, yielding named artifacts or precise reported failures.
- Retained controls, asynchronous source/executor changes, overlapping firmware exports, KiCad preparation/retry and final delivery preserve their source-specific ownership rules, including distinct download failures and preview refresh independence.
- Incomplete designs still save, reopen and edit, and unrelated valid outputs remain usable; exact geometry and existing provider limitations are preserved.
- Exactly the approved three tickets retain their dependencies and source evidence, and each implementation closure records changed native/browser targets, evidence and unmet criteria without treating child status as acceptance.

## Boundaries

- Touch point: Export presentation and output-specific admission seam — owner: epic-output-specific-export-admission; 001 establishes the seam, 002 and 003 extend their own output families.
- Touch point: Runtime/Session, export leases, builders and browser delivery adapters — consumed with existing authority and lifetime checks retained; owner: epic-output-specific-export-admission.
- Touch point: Firmware hardware plans, PCB wiring/preparation, outlines/footprints and accepted CAD geometry/providers — retain each domain's validation and output construction; owner: epic-output-specific-export-admission.
- Out of scope: Portable project export, Export styling, the independent Layout architecture candidate, missing providers, geometry algorithm rewrites, global completeness validation, fabrication guarantees and replacing ownership with UI readiness.

Tracer: mounted ZMK row → output-specific admission → real Runtime/Session → firmware builder → captured final delivery; 002 and 003 reuse that seam independently.

## References

- map — [frozen export-admission map](../export-admission/map.md), Tickets and Dependencies
- source companion — [001 ZMK export admission](../export-admission/issues/001-zmk-export-admission.md), all requirements, decisions and testing obligations; original progress/evidence stays here
- source companion — [002 PCB and design-file export admission](../export-admission/issues/002-pcb-design-file-admission.md), all requirements, decisions and testing obligations; original progress/evidence stays here
- source companion — [003 Geometry-package export admission](../export-admission/issues/003-geometry-package-admission.md), all requirements, decisions and testing obligations; original progress/evidence stays here
- research — [Candidate 02](../../research/2026-10-20-export-readiness-dispatch-deepening.md), Ownership must not be collapsed into readiness; Evidence and limits
- architecture policy — [ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md), [ADR-0014](../../adr/0014-document-authority-and-captured-lifetimes.md), [ADR-0015](../../adr/0015-exact-generated-cad-and-mesh-preview-scope.md), [ADR-0016](../../adr/0016-export-ownership-is-separate-from-preview-currency.md)
- verification and persistence policy — [ADR-0021](../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md), [ADR-0027](../../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md)
- terminology — [glossary](../../../GLOSSARY.md#mechanical-relationships)

## Notes

- Decision: 2026-11-15 — User approved one epic, Output-specific export admission, containing exactly legacy 001, 002 and 003 with their original titles. 001 is the tracer; 002 and 003 independently wait for 001. No extra story, refactor sweep or scope expansion.
- Decision: 2026-11-15 — All three legacy ready labels become ready-for-dev plans; dependencies still block 002/003. This migration does not start implementation or copy historical progress logs. Coordinator owns independent review and any commit.
- Decision: 2026-11-15 — No platform-baseline story is needed: this existing application already owns Runtime/Session, export builders and browser delivery. The source tickets preserve those owners.
- Decision: 2026-11-15 — The legacy plan contains no architecture.md; retain external ADR/research references and create no architecture companion.
