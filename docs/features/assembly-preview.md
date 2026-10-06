# PCB assembly previews

Layout's **3D assembly** combines the selected board with its component models
and the current case bodies. Visibility controls hide the PCB, copper, silkscreen,
models, keycaps, or individual case bodies without changing the document. Camera
controls provide top, bottom, isometric, and fit views. Parts → **3D model** shows
a footprint on a sample PCB; key presets open the assembly editor.

Assembly presets configure a switch, its socket where applicable, a diode, and
an optional LED. The switch generators name the *socket* side; presets use back
sockets to put switch housings above the board. The editor supports component
placement, side, multiple models, and model offset, rotation, and scale. Saving
an assembly does not rewrite existing placements. Explicit placement snapshots
its definitions; applying to a selected matrix retains its layout and key IDs.
Matrix replacements remap existing connections by logical terminal and reject
replacements that would merge different nets.
STEP/STP, STL, and static WRL models are supported. Imported assets and saved
assembly definitions travel with the project archive.

The **Routed PCB reference** inspector imports a `.kicad_pcb` as a read-only
assembly source. It replaces the generated board in the 3D view and does not
replace electrical editing/export data. Replacing a reference preserves its
alignment and model mappings. Referenced files can be attached individually or
matched by unique filename from a model directory. Source bytes and mapped
models persist in the project archive. Missing models produce notices while the
PCB remains visible.

Rust projects the KiCad board into revision-tagged contours, drilled holes,
front/back surfaces, and model placements. Generated previews use the same
export preparation and finalization path as board export. Model transforms
follow KiCad's local offset, clockwise ZYX rotation, scale, and bottom-side
frame. Case meshes retain their authored world elevations: visibility can expose
the PCB when an existing case or plate overlaps it.

## Preview limits

This is a visual projection, not a KiCad renderer or mechanical-fit check.
Outer copper, pads, tracks/arcs, saved zone fills, mask openings, silkscreen,
board cutouts, and round/oval holes are represented. It does not refill zones.
Silkscreen uses browser text, and unsupported custom pads may be approximated
with a diagnostic. Mask expansion, advanced text formatting, internal copper,
and material/finish details are not reproduced exactly. STEP/STL models without
colors use neutral shading; static WRL geometry/material colors are retained,
but external resources and executable nodes are rejected. Existing component
placements are not automatically migrated to the new presets.

## Implementation ownership

The Dioxus [Parts preview](../../web/src/presentation/parts/preview.rs) owns sample
request freshness, generation leases and local visibility.
[SharedViewer](../../web/src/presentation/shared_viewer.rs) projects Parts, Layout
and Case sources for the renderer and handles renderer lifecycle events.
[Runtime](../../web/src/runtime.rs) delivers scoped worker results; accepted
document edits remain in Session/Core. Export model packaging is a separate
operation.

The [assembly editor](../../web/src/presentation/parts/assembly_editor.rs) owns
editable Parts drafts. These presentation modules replace the retired React
`AssemblyViewer` and `ModelPreviewBoundary` owners. See
[architecture](../architecture.md) for the wider ownership map.
