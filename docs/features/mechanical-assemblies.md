# Library-backed mechanical assemblies

The Case workspace has an opt-in generated assembly. While enabled for a board,
the preview and stack editor show generated parts; disabling it restores the
authored-body preview and editor.
Enabling it stores a mechanical configuration in the v2 project; disabling it leaves
authored bodies intact. Configuration edits use the normal undo/redo history.

## Profiles and source geometry

Assign a mechanical profile to each switch definition. The bundled MX switch and
PCB-mounted 2u/6.25u stabilizer profiles extract their openings from pinned
Marbastlib footprints. The selected geometry is `Eco2.User`; drawing frames and
PCB drill holes are separate purposes. The pinned source, license, revision and
hashes are recorded under `core/tests/fixtures/mechanical/`.

The library supplies geometry, not a blanket manufacturing qualification. Confirm
the plate thickness and the **plate underside to PCB top** distance against the
selected component. The resolver derives the stack distance from assigned switch
profiles and rejects conflicting engagement values; this distance is user-specified,
not qualified by the cutout library. Custom plate-mounted stabilizers require an explicit custom
profile. Unsupported stabilizer sizes produce a diagnostic rather than a guessed
opening. Each key can override stabilizer type, size, orientation and custom
profile; its geometry follows the key placement.

For imported footprints, inspect the layer/primitive preview and explicitly assign
plate openings, PCB mounting holes, clearance envelopes or drawing guides. Only
closed selected openings become plate cuts. The retained source and mapping make
the selection reproducible after saving. Source-backed electrical footprints and
routed reference boards remain unchanged; missing PCB features are reported.

## Stack and manufacturing

The stack places the PCB top at Z=0. Plate foam must fit the configured distance to
the plate. Bottom depth includes PCB thickness, bottom foam and battery space.
Battery position, envelope and cable exit participate in local checks. Profiles can
supply foam exclusions, clearance volumes and connector/display access openings.
User-defined access openings retain their explicit geometry.

The shared plate geometry feeds PCB-fabricated FR4, printed, CNC and cut-sheet
outputs. An explicit radial opening allowance changes the manufacturing geometry;
the nominal functional contours remain available separately. A positive allowance
enlarges openings and a negative allowance reduces them. It requires fit review
and is never an automatic switch-engagement correction.

Tray, rigid and gasket mounting have separate suspension and closure settings.
Suggested mount locations are proposals; adopting them is explicit, and ordinary
regeneration preserves the chosen coordinates. Gasket plates remain separate from
rigid integrated frames. Material, method, stock thickness and the constraint
snapshot belong to the generated part process records.

Diagnostics distinguish blocking geometry conflicts from recommendations and
supplier review. The JLC PCB snapshot dated 2026-09-24 includes 0.5 mm minimum
NPTH, 1.0 mm minimum non-plated slots and unsupported sharp rectangular routed
openings. Material-web and wall recommendations do not constitute supplier
approval. See [JLC capabilities](https://jlcpcb.com/capabilities/Capab).

## Export and validation

Mechanical export captures one committed revision. It rejects changed or mismatched
results and includes individual STEP/STL parts, SVG/DXF outlines, an assembled STEP
reference and fabrication/critical-fit notes. The assembled PCB is a nominal
reference; it is excluded from the manufactured case parts. The FR4 option also
includes a separate mechanical KiCad plate board and project settings. It contains
closed Edge.Cuts and NPTH features, not copied electrical circuits. Use KiCad 10 to
generate Gerbers and drill files.

Hardware records attach a designation, thread, length, quantity and optional
tolerance/notes to a generated part's mounting feature. Critical-fit records
attach a labeled XY dimension and tolerance to a generated part. Both persist
with the configuration and appear in the fabrication notes and dimensioned SVG;
invalid or missing targets block export. Thread callouts specify manufacturing
intent; screw threads are not modeled in the solids.

The explicit KiCad integration gate is:

```sh
cargo test --manifest-path core/Cargo.toml mechanical_plate -- --include-ignored
```

It runs actual KiCad CLI Gerber and separate NPTH drill export. This gate complements
core extraction/stack tests, CAD volume and STEP roundtrip tests, app revision tests,
and browser workflows; it does not replace routed electrical or physical-fit review.

## Current limits and next steps

The **Case** workbench exposes **Configure mechanical stack**. The responsive
panels, local-save indicator, findings
navigation, Fit board/selection controls and bundled fonts remain shared with the
Layout and Parts workspaces. Generated geometry uses the normal project history;
manual case bodies remain in the document when generation is enabled or disabled.
The Export view distinguishes the generated mechanical package from authored Case
STEP output. One board owns the project's mechanical configuration; other boards
show their authored bodies and a link back to the configured board.

| Current boundary | Next step |
| --- | --- |
| Bundled profiles cover the MX switch and PCB-mounted 2u/6.25u stabilizer cutouts. Engagement distance is user-specified and supported stock-thickness ranges are not manufacturer-qualified. | Add manufacturer-backed engagement/thickness data and validated plate-mounted and additional-size profiles before expanding defaults. |
| Wide keys prompt for a stabilizer decision; size/orientation and custom plate-mounted geometry require explicit per-key configuration. Stabilizer component solids are not supplied by the cutout fixtures. | Add reviewed size/orientation proposals and independently licensed, correctly registered stabilizer models. |
| Mechanical hole checks compare references with existing electrical pad drills. Imported routed boards are read-only, and tray support holes still need electrical-board review. | Add an explicit reviewed reconciliation workflow for editable electrical boards while preserving read-only imports. |
| Component/battery interference uses conservative XY bounds and Z overlap, not complete solid collision detection. | Validate component, wall, fastener and stack-layer intersections with polygon/solid geometry and representative assemblies. |
| Battery foam uses a conservative cable-access envelope; battery retention and detailed cable routing are not generated. | Add configurable retention/strap features and cable path/connector clearance checks. |
| Gasket mounting requires a shell bottom; sheet/frame gasket seats are rejected. A separate middle frame requires a sheet bottom and separate plate. | Add tested mating seats and travel clearances before enabling more combinations. |
| Manufacturing checks use a dated local constraint snapshot. Kerf, shrinkage and cutter compensation are not automatic; opening allowance is explicit. Thick-stock engagement reliefs are not generated automatically. | Add versioned supplier/material profiles, measured process coupons and profile-defined engagement reliefs with fit tests. |
| Exact source primitives are retained, but built-in operational cutout contours use bounded tessellation (maximum chord deviation 0.005 mm). | Add analytic-curve manufacturing output where a process needs it, while preserving nominal/adjusted parity. |
| Assembled STEP includes generated parts and a nominal unpopulated PCB. Component solids are preview assets rather than populated STEP members. | Add component-solid placement/export with front/back, mirrored and rotated assembly roundtrip tests. |
| Hardware/thread specifications and critical-fit XY dimensions are persisted callouts. Threads are not modeled, and tolerances are user-entered text. | Add hardware catalogue validation and additional section/depth drawings when required by the selected machining process. |

Direct Gerber serialization, bent-sheet enclosures, quoting and ordering remain
outside scope. KiCad generates fabrication files from the separate plate project;
passing the automated gates does not establish supplier approval or physical fit.
