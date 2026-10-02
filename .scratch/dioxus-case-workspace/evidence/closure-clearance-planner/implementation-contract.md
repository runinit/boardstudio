# Private F7.4e closure clearance planner

Status: implementation draft; module is intentionally unmounted pending the
root Case controller integration. This evidence covers only a private pure
document-planning helper. It does not accept the F7.4e workflow, close F7.4,
or replace the required INT.2 integration join.

## Boundary and API

`web/src/presentation/closure_clearance.rs` provides
`project_closure_clearance(document, mounting_hole) -> ProjectDoc`.
The caller supplies the current accepted canonical document after its
configuration/link-policy update and the existing normalized bundled
`ceoloide/mounting_hole_npth` definition. The helper reads canonical and
physical-instance closure configurations and returns the complete derived
projection. The caller remains responsible for building the field patch,
applying the established canonical/instance policy, admission, submitting one
`ReplaceDocument`, and checking the saved acknowledgement.

The planner performs no resolution, validation, geometry generation, default
selection, or suggestion initialization. It does not access Runtime, browser
state, CAD, or the shared catalogue. The supplied catalogue definition carries
the checked-in generator identity and normalized footprint metadata. The
planner updates the generator's `hole_size` / `hole_drill` parameters and its
single NPTH pad and derived pad-bounds courtyard to match the exact generated
mounting-hole source contract.

## Source-backed behavior

The projection follows `app/src/closureClearance.ts::withClosureClearance`:

- Include canonical mechanical configuration and all configured physical
  instances. Reflect X for flipped instances before deduplication.
- Deduplicate by board and JavaScript-compatible five-decimal XY keys,
  retaining the latest exact position and the largest required diameter while
  preserving the key's first insertion order. Bosses use
  `bossDiameter ?? holeDiameter` plus twice configuration clearance; screws
  use their hole diameter.
- Replace only old `case-closure/` parts and
  `assembly-closure/definition/` definitions. Generate project-owned parts
  with the checked-in mounting-hole generator source and parameters.
- Allocate `MHn` after the maximum reference found among remaining
  non-generated parts. Remove stale generated membership from layouts and
  rebuild generated board membership while preserving remaining order and
  document fields.

The implementation intentionally returns a complete cloned `ProjectDoc` so
the root Case controller can put the configuration update and its derived
projection into one existing Session edit. No public API, schema, Rust Core,
CAD, library-catalogue, or shared component changes are part of this helper.

## Evidence and limits

Source review: `app/src/closureClearance.ts`,
`app/src/closureClearance.test.ts`, the published issue
`.scratch/dioxus-case-workspace/issues/08-mechanical-clearance-projection.md`,
and the corrected settings contract under
`evidence/mechanical-settings-contract/implementation-contract.md`. The
React helper obtains its default normalized definition from the existing
Ergogen catalogue and calls `normalizeDefinition` after changing generator
parameters; the checked-in source `ergogen/library/mounting_hole_npth.js`
defines one circular non-plated pad whose size and drill follow those two
parameters. The Rust helper consumes the pre-normalized catalogue definition
and preserves its metadata while updating that source-derived pad and bounds.

Native unit tests cover canonical plus flipped-instance deduplication with
unequal boss diameters and latest-position retention, JavaScript rounding and
negative-zero behavior, five-decimal boundary handling, non-plated normalized
definition fields, large valid MH reference allocation, and screw-diameter /
explicit-empty cleanup. The root task owns compilation and test execution; this
evidence records no passing test claim. Those checks
do not establish controller admission, session transaction, persistence, Undo
/ Redo, archive reopen, browser visibility, or public workflow acceptance.

## RF-005 handoff

Record this frontend planning coupling under existing RF-005: React translates
canonical and physical-instance Case closure intent into generated NPTH
project parts, generator-backed definitions, layout cleanup, and board
membership, while Core `SetMechanical` only assigns configuration. Keep this
narrow deterministic projection private and submit it through the existing
`ReplaceDocument` / Session edit path; Core remains the authority for
validation, CAD, and export. This is source confirmation of RF-005's existing
policy-ownership concern. No additional refactoring takeaway is claimed.
