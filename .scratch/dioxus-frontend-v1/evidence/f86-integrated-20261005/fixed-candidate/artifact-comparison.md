# Fixed candidate artifact comparison

This is a static comparison of the retained candidate and reference mechanical exports. The authored Case STEP is a separate action and is reported independently because the reference authored export was disabled.

## Mechanical package

- Candidate ZIP: 2,524,362 bytes, SHA-256 `cf8be458114ed0a660691144806ef53198e6cb0e7883affe3143cf52e6035381`; ZIP integrity passes.
- Reference ZIP: 2,524,303 bytes, SHA-256 `d307d32cd341f7a939438e607a6b0e3fb7a2fd4afc5e6a3f927097156ca6c65a`; ZIP integrity passes.
- Both contain the same 20 entry names. Raw hashes and lengths for every entry are in the JSON record.
- Each `assembly.json` revision matches its own joined project archive: candidate 24/24 and reference 23/23. The parsed manifests differ at seven paths, including revision, four stack z/thickness values, and one diagnostic identity/target. Exact candidate/reference values are retained in JSON; this packet does not attribute the differences to a specific action.

## Generated geometry and text

- All five paired STEP entries have valid ISO-10303-21 start/end signatures and equal 3D Cartesian-point multisets after rounding to 1e-8 mm. Raw byte hashes differ; coordinate agreement is not a B-rep topology identity claim.
- All four binary STL pairs parse with matching triangle counts and unique-vertex counts. Bounds match exactly; maximum surface-area and signed-volume deltas are below `1e-10` in the corresponding mm units. Vertex and triangle occurrence multisets differ at 1e-6 mm, so the triangulations are not claimed equal.
- All nine SVG/DXF files compare equal after line-ending, whitespace/blank-line, and 10-significant-digit numeric normalization. Raw hashes are retained and are not treated as byte equality.
- `FABRICATION.md` differs after normalization only at `Revision: 24` versus `Revision: 23`; that emitted revision difference is retained, not normalized away.

## Authored Case STEP

Candidate `candidate-authored.step` is 976,779 bytes (SHA-256 `7db09ddd4b46c60f7868b1d7da1a3a038ac0d787d1311eb9e4303510b7d23253`), has valid start/end signatures, 1,539 parsed 3D Cartesian points, one `MANIFOLD_SOLID_BREP`, and one `CLOSED_SHELL`. This is the authored Case geometry output, separate from the configured-stack ZIP. The reference action was disabled in its retained state, so no paired authored STEP exists; generated-package agreement cannot stand in for that missing comparison.

## Limits

This is static artifact inspection. No CAD kernel, fabrication checker, or watertightness/topology validation ran. Numeric STEP/STL comparisons use the tolerances above; SVG/DXF results are normalized-text equality, not byte equality. Candidate revision 24 versus reference revision 23 remains an unexplained single Parts-group revision in the broader journey; this packet does not assign its cause.
