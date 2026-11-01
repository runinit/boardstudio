# 15: Trim CAD test dependencies

Status: wontfix
Type: research
Blocked by: none
Parent: [map](../map.md)
Related: [07](07-inflector-regex.md), [09](09-cad-owner-feature-set.md)

## Question

Can the qualification tree shed `clap`, `proptest`, `vtkio` and similar dev-dependencies for 20 to 40 CPU-s?

## Answer

Only by forking more vendored code, for little gain. Do [07](07-inflector-regex.md) and
[09](09-cad-owner-feature-set.md) instead. They remove more from the same tree and leave the vendored tests intact.

## Evidence

Cost in `cad/monstertruck-qualification/target`, today, with crates built in up to 2 variants ([09](09-cad-owner-feature-set.md)
halves these):

| Family | CPU-s | Needed by | Removable? |
|---|---:|---|---|
| `image`, `moxcms`, `pxfm`, others | 27.5 | `monstertruck-io/examples/preview-step-face.rs` only | Only by deleting the example |
| `vtkio` and its parsers | 17.9 (`nom` also serves `step-p21`) | meshing's optional `vtk`, enabled through a dev-dependency (`monstertruck-io/Cargo.toml:228`); meshing `tests/vtk.rs` | Needs a vendored manifest edit |
| `proptest` family | 12.6 | `monstertruck-io/src/step/load/step_geometry/sphere.rs`, `tests/input/geometry.rs`, meshing `tests/analyzers/volume.rs` | **No**: `--test input` is a gated selection |
| `clap` family | 11.4 | Five `monstertruck-io/examples/*.rs` | Only by deleting the examples |

- Cargo cannot make a dev-dependency optional, and a selection such as `--test input` still compiles every
  dev-dependency in the package. `required-features` on examples would not help.
- Round one already narrowed `image` to PNG (`vendor/monstertruck-io/Cargo.toml`). `clap` and `image` are used only by
  examples that no gate builds, but deleting vendored upstream code makes each future vendor sync harder.
- Attributed wall: 9.8 s for the wider set of 44 units, on the CAD lane.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -2 to -5 s | -20 to -50, or about half that after 09 | none |

The upper end needs `vtkio` removed too. Effort: half a day. Risk: low to medium. Larger vendored diff; the
[closure rule](../../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user) applies, with
RED/GREEN evidence and a diff read.

## Reopen when

The CAD qualification lane is still the wall of `test` after 04 and 09, and the vendored examples are unwanted anyway.

Verification then: `cd cad/monstertruck-qualification && cargo tree --locked --offline --workspace -i clap -e
normal,build,dev` finds nothing, then `python3 cad/scripts/test-cad.py` and `python3 scripts/check.py test` with all
test names unchanged. Stop rule: revert under 25 CPU-s measured.
