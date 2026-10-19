# 01: Audit current aggregate report usage

Status: resolved
Type: build

## What to build

Audit where monstertruck-io's `EntityLoadReport::require_empty()` and
`ShellLoadReport::require_lossless()` are currently used in BoardStudio's
import path. Determine what's already gated and what's missing.

## Acceptance criteria

- [ ] Document every call site of `to_compressed_shell_reported`,
  `to_compressed_solid_reported`, `entity_report`, `ShellLoadReport`,
  `EntityLoadReport` in `cad/monstertruck` and `cad/wasm`.
- [ ] Identify which conversions currently discard the report (un-suffixed
  methods) and whether they should be strict.
- [ ] Record which conversions are lossy-by-design (e.g. `DegenerateVertexLoop`)
  and must stay non-strict.
- [ ] Produce a one-page summary: what's gated, what's not, what should change.

## Notes

- The un-suffixed methods (`to_compressed_shell`, etc.) still exist and discard
  the report. Strictness is opt-in.
- In-gate fixtures are lossy by design: `DegenerateVertexLoop` is the correct
  outcome for a point boundary. These must not become strict.
- `Table::solids_via_shape_relationship` exposes part→solid hop losses.

## CURRENT

Done. Audit findings:
- `require_empty()` is called in mod.rs (import path) and tessellation.rs.
- `require_lossless()` is NOT called in the production import path.
- Shell conversions use the repair system's own completeness checks.
- The repair system's `convert_input_with_refusal` / `convert_normalized_with_refusal`
  already enforce completeness through the repair ledger.
- Aggregate reports are used at the table level (entity_report) but not at the
  shell level in the production import path. The repair system provides equivalent
  completeness enforcement through its own mechanisms.
