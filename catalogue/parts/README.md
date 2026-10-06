# Reviewed KiCad parts

The reusable library includes two approved marbastlib MX PCB stabilizers (2u and
6.25u). Source files and their CERN-OHL-P-2.0 license are retained under `sources/`.
These parts have four non-plated PCB holes and two plate openings each. Drawing
layers remain guides; they are not plate cuts. The importer reports a conservative
preview envelope because these sources have no closed courtyard.

The stabilizer profiles explicitly use the MX switch family. The resolver derives
the underside gap from the 5 mm mounting datum minus the configured plate
thickness (3.5 mm for a 1.5 mm plate). Existing saved catalogue profiles with the
original 5 mm value and exact reviewed source receive the same interpretation
without changing the saved document; unrelated custom engagement values remain
authoritative.

## Importing another part

Confirm any new component with the user before adding it to this library.
Reuse existing generator definitions where possible.

1. Retain the upstream `.kicad_mod` and license. Add an entry to
   `import-manifest.json` with its pinned repository revision and SHA-256.
2. Explicitly review the kind, terminal roles, and mechanical purpose mappings.
   Terminal values are arrays of physical pad-number strings. Duplicate numbered
   copper pads map to the same role; NPTH holes cannot become terminals. Optional
   `matrixTerminals` names the reviewed row and column roles. No electrical
   function is inferred from a footprint's geometry or name.
3. Build the existing Rust artifact driver and generate the catalogue:

   ```sh
   cargo build --manifest-path core/Cargo.toml --locked --example artifact_request
   python3 scripts/import-kicad-parts.py
   python3 scripts/import-kicad-parts.py --check
   ```

4. Review the generated definition, diagnostics, geometry, and electrical mapping.
   Add regression coverage for the new part before using it in a demo.

The script also accepts a manifest path and output path for isolated conversions.
It rejects changed source hashes and duplicate definition IDs before publishing
output. It uses Board Studio's Rust importer rather than a second KiCad parser.
Exact source is preserved in `kicadSource`; the normalized pads and envelope serve
the editor while export retains source geometry. Mechanical layers require
explicit purpose mappings. A source import alone does not establish electrical
compatibility or manufacturing readiness.

For one-off project parts, Board Studio's existing Import Part action accepts
KiCad footprints. Adding a project part does not update this reviewed library.
