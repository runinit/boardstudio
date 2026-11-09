---
status: accepted
---

# Distinguish collision observations from supported clearance conclusions

Collision checks should show useful approximate measurements and potential-interference findings, including for repaired or incomplete vendor previews, without requiring universal mathematical certification just to run or display a check. A clearance conclusion at a requested accuracy requires evidence supporting that accuracy; insufficient evidence remains explicitly uncertain, not “clear.” The user confirmed this policy on 2026-10-19, choosing useful approximate feedback without misrepresenting it as verified physical clearance.

An incomplete model cannot establish clearance for the whole model, and finer meshing alone does not establish source accuracy. Report the limitations of the geometry actually checked; do not present a measured mesh gap as a guaranteed physical gap. This complements [ADR-0017](0017-forgiving-vendor-previews-with-preserved-source.md)'s forgiving preview admission and leaves [ADR-0015](0015-exact-generated-cad-and-mesh-preview-scope.md)'s generated manufacturing scope unchanged. It records an output-honesty policy, not a mandate for a particular certification framework or a claim that the newly accepted preview behavior is implemented.

Historical context: [collision decision and implementation record](../plans/cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md). Earlier proof-oriented plans do not override the forgiving preview and approximate-check admission accepted here and in ADR-0017.
