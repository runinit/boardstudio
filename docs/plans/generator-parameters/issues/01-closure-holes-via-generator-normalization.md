# 01: Closure clearance holes through generator normalization

Status: ready-for-agent
Type: build
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0003](../../../adr/0003-rust-footprint-generators-in-core.md)

## Request

`normalized_mounting_hole` (`core/src/mechanical/clearance.rs:198-222`) writes
`hole_drill`/`hole_size` as `Value::String`. `ceoloide/mounting_hole_npth` declares them
as numbers (`footprints/src/generators/ceoloide/mounting_hole_npth.rs:17-22`), and the
registry rejects them only at render time (`footprints/src/registry.rs:152-158`):
"parameter hole_size must be a number". Any half that keeps its proposed closure mounts
fails preview and export. Lines 223-284 also recompute the pad and courtyard by hand,
duplicating the generator's own normalization. Callers: `clearance.rs:35,48`,
`core/src/mechanical/edit.rs:345-363`.

## What to do

- Write both parameters as JSON numbers, then call
  `crate::generators::normalize_definition` instead of the hand-written pad/courtyard
  code. Keep the template's id, name, kind and models; keep the definition id prefix,
  part ids, MH references and memberships.
- Normalization errors propagate as `Err(String)` before anything changes.
- Update the unit test (`clearance.rs:681-682`) to expect numbers and whatever the
  normalizer produces for pads, courtyard and envelope; record any field that differs.
- Add a regression test in `core/tests/`: open a project, enable a half so closure mounts
  are proposed, keep them, then render every generator-backed definition and part
  (`generators::render_forms`) and run board preview and KiCad export through the
  engine. Cover a boss diameter that is not a clean decimal (2.8000000000000003), a
  flipped instance and a linked pair.
- Re-bless the boundary transcript (`BOUNDARY_BLESS=1`). Only the 8 lines at
  `core/tests/fixtures/boundary/transcript.json:15685-16854` may change; diff to confirm.
  The transcript is checked natively and in WASM, so both must pass.

## Acceptance

- [ ] No `Value::String` is written for numeric generator parameters in `clearance.rs`.
- [ ] Hand-computed pad/courtyard code is removed; normalization is the only source.
- [ ] The regression test fails on the old code and passes on the new.
- [ ] The transcript diff is limited to the hole parameters (plus normalizer fields, if
      any, listed in the Outcome).

## Verification

`python3 scripts/check.py quick`; Core native tests (non-nested checkout); boundary
parity native and WASM. GitNexus `impact` on `normalized_mounting_hole` and
`project_with_optional_template` first.

## Risks / ADR

Normalization could change pad ids or courtyard text, which would mean more transcript
churn. No ADR: this applies ADR-0003. Out of scope (see the map): Core building the
template itself and removing the `mountingHole` wire field.
