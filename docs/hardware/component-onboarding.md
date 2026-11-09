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

## Generator parameters

A footprint generator declares each parameter once, where the generator is written
(`footprints/src/generators`). The declaration is the contract: a saved value must have
the declared kind, and Core checks every saved value against it
([ADR-0009](../adr/0009-generator-parameters-checked-in-core.md),
[ADR-0003](../adr/0003-rust-footprint-generators-in-core.md)).

- **Declare the kind explicitly.** Use `number`, `boolean`, `string` (text), `net` (a
  net name; empty means no net) or `array` (a JSON list, such as a model offset). No kind
  is inferred from a default. There is no integer or enum kind yet; declare a count as a
  number.
- **Millimetres, as numbers.** A length, drill, diameter or offset is a JSON number in
  millimetres. Text such as `"2.2"` is not a number: Core refuses it on an edit, and a saved
  file that holds it opens with a warning.
- **No numeric strings.** A text parameter must not have a numeric-looking default or end in a
  dimension word (`size`, `drill`, `width`, `diameter`, `offset` and so on). The
  [parameter lint](../../footprints/tests/parameter_lint.rs) reports both.
- **`side` on every generator.** Each generator declares `side` as text with default `F`. Core
  writes `B` for the effective read of a half that a mechanical mirror flips.
- **Names are snake_case.** Two kinds of name keep a pad's own name because saved documents
  hold it: a `net` parameter named for its pad or pin in capitals (`GND`, `P21`, `BAT_P`), and
  the text parameter `<PAD>_label` that labels that pad's pin (`P21_label`). The lint accepts
  them by their declaration, not from a list: only a `net` may be named in capitals, and a
  label needs a declared net of that name. It pins how many of each the bundled generators
  declare, so adding one is a reviewed change to that count. Any other new name is snake_case.
- **Duplicate names, defaults and `side`** are also checked when the registry is built, so a
  wrong declaration stops the program at start. The lint reports them all in one run.
- **A change of kind is a migration.** Changing what a saved value must be (for example,
  number text saved as text) needs a versioned migration step, as
  [ADR-0004](../adr/0004-document-format-version-and-id-rename.md) requires. Version 4 is the
  example: it converts saved numeric text to numbers. An unversioned rewrite on each load is not
  allowed.
- **Core's check.** Open and ReplaceDocument refuse a wrong type, and warn on a name the
  generator does not declare and on text where a number or a list is declared. An edit refuses
  every value it writes that is wrong ([ADR-0009](../adr/0009-generator-parameters-checked-in-core.md)).

Legacy vocabulary is kept, not renamed, because saved documents and golden schemas hold these
names; renaming and new kinds are out of scope for the generator-parameters work:

| Generator (or owner) | Legacy names |
| --- | --- |
| Non-plated mounting hole (`ceoloide/mounting_hole_npth`) and closure mounts | `hole_size`, `hole_drill` |
| Plated mounting holes (`ceoloide/mounting_hole_plated`, `infused-kim/mounting_hole`) and the trackpoint mount | `drill`, `outline` |
| Diode, LED, MCU, switch (MX, Choc) and utility router generators | `via_size`, `via_drill` |
| Case mount fields (not generator parameters) | `hole_diameter`, `boss_diameter` |

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
