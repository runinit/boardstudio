# 02: Drop GVP comparison (superseded by research)

Status: done
Type: build

## What to build

~~Add GVP comparison.~~ Dropped per research (`docs/research/cad/gvp-necessity-rust.md`).

ISO 10303 does not require GVP for import admission. No real vendor files in
the corpus carry GVP. No Rust crate parses STEP GVP. OCCT treats GVP as
advisory, not a gate. Aggregate reports + invariants + budget gate are sufficient.

## Acceptance criteria

- [x] GVP code removed (extract_gvp_volume_mm3, mesh_volume_mm3, gvp_* fields).
- [x] Research documented at docs/research/cad/gvp-necessity-rust.md.
- [x] Spec and map updated to reflect drop.

## CURRENT

Done. GVP comparison dropped. Aggregate reports + invariants + budget gate
remain as the validation approach.
