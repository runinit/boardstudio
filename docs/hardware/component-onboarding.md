# Footprint and component onboarding

Read this guide before adding, importing, or changing a library component,
footprint, module, or associated 3D model. The PCB presentation demonstrated by
the VIK review project is the convention for **all component classes**, including
switches, encoders, controllers, connectors and constituent circuit parts.
Existing entries that lack source artwork need an explicit follow-up; adopting
this guide does not establish that every current entry already complies.

## Onboarding workflow

1. **Record source evidence.** Identify the exact part and variant, pin mapping,
   source URL/revision, licence, units and asset hashes. Keep source assets
   reproducible and portable. Record reversible configurations and front/back
   connectivity separately; a mirrored drawing alone does not prove reversibility.
   Separate measured/source facts, designer configuration and unknown values.
2. **Choose ownership.** A placed host component owns host pads and nets. A mounted
   daughterboard keeps its constituent footprints and copper source-owned; its
   preview does not create host circuitry. An embedded circuit copy receives
   independent host identities and explicit electrical joins. Reuse existing
   library assets and connectors. Host connectors are real movable parts, linked
   to their module and initially placed nearby without changing reviewed anchors,
   manual placements or authored outline versions.
3. **Integrate through Rust.** Use the public edit/resolution/export boundaries
   and shared Rust contracts. Persist edits through the normal validation and Undo
   path. Carry geometry changes through CAD, cache identity, preview and affected
   exports. Use [the architecture owners](../architecture.md) and the project
   Rust/CAD integration skill rather than adding a parallel geometry authority.
4. **Provide PCB artwork.** Render authentic footprint pads, drills, silkscreen,
   fabrication graphics and reference text with the source layer, width and local
   geometry preserved. Compose placement, rotation and face transforms once;
   consume resolved front/back layers without applying a second face swap.
   Use the layer rules below for every component class.
5. **Provide mechanical integration.** Verify model units, bounds, orientation
   and datum against the footprint. Asset bounds alone do not establish alignment
   or assembly fit. For mounted modules, retain source mounting-hole identities
   and configure host drills and annular standoffs explicitly. Account for above/
   below placement, installed hardware, cable/service space and case intersections.
   Keep source holes, host drills and support bodies distinct.
6. **Demonstrate acceptance.** Verify the applicable rows below with public
   behavior tests, exported geometry and an editable app example. Record missing
   facts as output-specific findings/gates. Safe incomplete previews may show
   known geometry with diagnostics; fabrication readiness requires the necessary
   evidence. Physical QA can be deferred explicitly; tests and human app review
   must remain separately identified.

## PCB layer rules

| Geometry | Presentation and controls |
| --- | --- |
| Footprint pads, drills, courtyards and references | Follow the existing object controls and relevant copper faces; references appear once. |
| Source silkscreen | Visible by default, with independent front/back controls. |
| Fabrication artwork | Independent front/back controls; module fabrication art starts hidden. |
| Module board outline, clearance/service envelope, source mounting holes, standoffs and findings | Separate controls, hidden by default in PCB mode. |
| 3D body or coarse bounding rectangle | Use assembly/mechanical views; a body rectangle is not substitute footprint artwork. |

Toggling one layer must leave unrelated geometry and document data unchanged.
Selection of source-owned footprint geometry opens its owning module; host
footprints select their own part. Finding navigation reveals and focuses the
specific affected point/region even when its ordinary overlay is hidden.
Layers control presentation, not fabrication contents or validation.

## Required evidence

| Boundary | Acceptance evidence |
| --- | --- |
| Source and library | Reproducible variant/assets, correct pin identities, portable saved-project round-trip. |
| PCB | Default footprint presentation, independent layer controls, front/back and rotated placement, owning-object selection, precise finding focus. |
| Edits | Undo/Redo and reload preserve identities, settings, manual placements, unrelated wiring and authored outlines. |
| Export | Actual footprint artwork, pad/drill coordinates and layers match the resolved placement; mounted daughterboard copper stays separate from host output. |
| Mechanical, when applicable | Independent model bounds/alignment evidence and actual exported solids/drills; support contact, thickness, gap and above/below extents checked. |
| Electrical/firmware, when applicable | Explicit host assignments and supported behavior through public APIs; unknown mappings or driver facts retain their gates. |

Use the [archived VIK review scenarios](../archive/vik-module-human-review.md) as a
presentation example and [VIK source notes](vik.md) for its unresolved facts.
The archived React test commands need current Dioxus equivalents before execution.
Imported, previewable and fabrication-qualified are separate claims; record the
evidence for each supported output before marking an entry complete.
