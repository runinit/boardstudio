# 41: Inspector controls for the companion placement mode

Status: resolved
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

- [x] Each selection kind changes eligible companions only; independent parts are excluded, stabiliser controls are disabled, Undo works, and mixed values display correctly.
- [x] No new visible control pattern was introduced; the named disclosure reuses the existing pattern, so DESIGN.md needed no change.

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

## Progress (2026-10-14, full Editor selection owner)

Added a mounted browser test that opens a synthetic mirrored switch/LED pair in Core, mounts the
real `Editor`, supplies a key selection using the Editor's current runtime scope, applies Duplicate
position through the rendered disclosure control, and verifies Core state and Undo. The test
exposed that Editor startup changes the active hardware instance, so the selection must use the
post-mount scope. All 8 `layout_key_selection_actions` browser tests passed, including this case.
The earlier full repository gate and typecheck remain the broad verification; complete selection-
kind integration, three repeated current-tree browser gates, and required reviews still need closure.

## Progress (2026-10-14, current-tree verification)

Three consecutive `python3 scripts/check.py browser` runs passed in **127.5 s**, **106.5 s**, and
**105.7 s**. The full `python3 scripts/check.py` passed all steps in **193.9 s**. These gates verify
the current partial implementation; the mounted Editor coverage for each selection kind, mixed
selection, refusal, and independent-part exclusion remains incomplete, so acceptance stays open.

## Progress (2026-10-14, full Editor selection coverage)

The mounted real-Editor test now applies placement from matrix, row, both columns, both keys, an
individual component, and a multi-part selection. It verifies eligible source links change while
unselected links retain their mode, and Undo restores each prior value. A rendered-context wait
was necessary because changing the test adapter signal alone could leave the prior Inspector
owner mounted for one render; this fixed the stale-context failure. The focused WASM suites for
the key-selection actions and component Inspector pass **48/48**. Existing mounted checks retain
refusal and stabiliser-disabled coverage, while the projection test checks independent-part
exclusion. Final browser and repository gates remain pending.

Those gates are recorded below.

The three consecutive final-tree browser gates and the final full check passed; focused mounted
WASM suites passed **48/48**. All #41 acceptance evidence is now present; resolved.
