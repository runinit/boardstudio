# 41: Inspector controls for the companion placement mode

Status: claimed
Type: build
Blocked by: [40](40-set-companion-placement-operation.md), [21](21-mirrored-half-web-adapter.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

User, 2026-10-10: the Inspector panel edits the mode. The later human clarification limits
this to **switch companions only**: an individual companion, companions in a row/column or
matrix, and several selected companions. Independent controllers/connectors keep the general
mirror rule and do not gain placement-mode controls.

## What to do

- In the Layout Inspector, for eligible switch companions or key/matrix selections that identify companions,
  show **Companion placement** with literal mirror (default) / duplicate position / mirror with tweak
  (x, y, rotation fields), reflecting mixed values for a multi-selection. Independent parts are excluded.
- Build the `targets` list for one/several companions, companions in a selected row/column or matrix, and the
  pair default. Normalize source-side or mirror-side selection to 40's `(kind, mirrorId)` companion identity;
  deterministically deduplicate overlaps. Submit `SetCompanionPlacement` through the typed edit ticket path;
  a pending edit keeps the draft value (ADR-0005). Stabilisers show the control disabled with the reason:
  their mirrored-X position and unchanged heading follow 39 in every mode.
- Pair-default tweak controls must follow 40's root-reviewed technical contract; do not introduce a pair-wide
  tweak or independent-part coordinate semantics in the adapter while that review is pending.
- Follow DESIGN.md for controls and copy; read it before changing the panel. Mounted tests for each selection kind, a mixed selection,
  Undo, a refused edit showing the inline message, and exclusion of independent controllers/connectors.

## Acceptance

- [ ] Each selection kind changes eligible companions only; independent parts are excluded, stabiliser controls are disabled, Undo works, and mixed values display correctly.
- [ ] DESIGN.md gets one sentence only if a new visible control pattern is introduced.

## Verification

`repo lint typecheck browser`; mounted tests use the shared `wait_until` helper.

## User decision (2026-10-14)

Present the control in a named disclosure section, **Companion placement**.

The user also approved a narrow read-only Core eligibility projection for stabilisers so the
Inspector can display Core's disabled reason without duplicating the domain rule. The projection
accepts either the source or mirror part id; mounted Inspector integration and coverage remain open.

## Progress (2026-10-14)

- `20ce8e8c` adds the named Companion placement disclosure, typed `SetCompanionPlacement`
  submission, pending/error handling, source/mirror normalization, and Layout projections for
  matrix, row, column, key, component, and multiple selected companion contexts. Independent
  parts do not become placement targets; Core's stabiliser reason disables the mode control.
- WASM tests cover target projection across those contexts, mixed modes and independent-part
  exclusion, typed action emission, refusal-message rendering, and the disabled stabiliser state.
  `repo`, `lint`, `typecheck`, the focused browser suite (36 tests), `cargo fmt --all --check`,
  `git diff --check`, and `impeccable detect --json` passed.
- Acceptance remains open: the mounted tests do not yet exercise selection through the full
  Editor. The pair-default contract is recorded and integrated verification is tracked below.

## Progress (2026-10-14, mounted Core edit)

The mounted Companion placement control now drives the typed resolver through Runtime and Core.
The test changes the pair default, waits for the accepted revision, undoes it, then injects an
invalid pair-plus-tweak request and verifies Core's refusal appears inline while accepted state
stays unchanged. A separate mounted check confirms mixed modes and tweaks render as “Mixed values.”

`python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs`:
39 passed. Formatting passed. These tests use the mounted control with an in-process Runtime/Core
host; full Editor selection-owner integration remains open, as does running `repo lint typecheck
browser` at this ticket's acceptance scope.


## Progress (2026-10-14, current repository gate)

The current `python3 scripts/check.py` gate passed all steps in 202.4 s, including repo checks, lint, build, native tests, and browser tests. This verifies the current partial implementation; full Editor-owned selection integration and its acceptance tests are still missing, so #41 remains open. The explicit `python3 scripts/check.py typecheck` step also passed in 5.7 s.
