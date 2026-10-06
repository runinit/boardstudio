# Board Studio v2

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Experienced keyboard designers working with keyboard components, layouts,
KiCad, and CAD tools. They need precise control and fast iteration without
routine tasks being buried among rarely useful settings.

## Product Purpose

Board Studio v2 is a visual design workbench for arranging keys and components,
deriving board and case geometry, and exporting coherent artifacts for downstream
work. Success means a designer can iterate on a layout, understand its effect on
the board and case, and carry a consistent design into KiCad and CAD tools.

## Positioning

Direct manipulation works alongside parametric matrices, constraints, and shared
geometry. The same resolved board contours feed the layout preview, PCB edges,
outline exports, and case construction. Visual editing and generated geometry
are parts of one workflow.

## Operating Context

- The primary working context is desktop editing, with keyboard controls and
  responsive access on narrower screens.
- Projects persist locally and can be saved and reopened as portable project
  files. The app shell and browser storage support offline work after initial
  loading; optional model assets and the CAD kernel load on demand.
- The workbench connects Design, PCB, Case, Parts, and Export. Parts provides
  compiled footprint previews and attached model previews.
- Designers continue routing and final electrical checks in KiCad, and review
  mechanical fit using the exported geometry and downstream tools.

## Capabilities and Constraints

- Preserve stable document, part, footprint, and net identities; undo/redo;
  precise millimetre measurements; and revision-consistent previews and exports.
  Exporters must reject stale results.
- Support direct placement and parametric matrices, component libraries,
  constraints, automatic outlines and authored cutouts, net assignments, and
  plate, tray, and lid case geometry.
- Keep common placement and layout controls immediately available. Disclose
  useful specialist options in named sections. Standard footprints retain
  sensible geometry defaults; raw footprint editing belongs to custom or
  imported parts. Preserve previously saved supported parameters.
- Project files use the `boardstudio/v2` format. There is no v1
  project importer or conversion path.
- Bundled Ergogen generators are trusted inputs. User-supplied JavaScript
  generators are not executed. Footprint import supports a documented subset.
- Preview availability, export validity, and fabrication readiness are distinct.
  A 3D preview does not establish clearance or fabrication readiness. Routed
  board review and mechanical-fit verification remain downstream requirements.

## Brand Commitments

The product name is Board Studio v2. Use the established workflow terminology:
Design, PCB, Case, Parts, Export, components, key assemblies, matrices, and
outlines.

## Evidence on Hand

- [The v2 overview](README.md) records capabilities, limitations, commands, and
  package boundaries.
- [The starter keyboard](core/examples/demo_projects/review.rs) provides a working layout example.
- [The bundled library](ergogen/library/) contains real generator sources,
  model assets, and vendor provenance. Preserve their attribution and terms.
- [Core tests](core/tests/), [KiCad integration tests](core/tests/kicad_integration.rs),
  [CAD tests](cad/wasm/src/model/), and [mounted browser tests](web/src/presentation/) provide implementation
  evidence. Their existence does not imply a current passing run.
- Performance validation (historical record in Git at `323967ff`) records measured
  fixtures and conditions. Do not generalize those results into unsupported
  performance or fabrication claims.

## Product Principles

1. **Precise direct editing.** Make spatial changes predictable, measurable, and
   reversible, with keyboard access alongside pointer manipulation.
2. **Focused expert controls.** Put frequent decisions first, use dependable
   defaults, and reveal specialist controls where they help the current task.
3. **Consistent geometry.** Keep views and exported artifacts tied to the same
   resolved design revision.
4. **Local ownership and portability.** Keep projects usable locally and
   transferable through explicit project files and interoperable exports.
5. **Explicit readiness and limitations.** Explain pending work, failures, and
   evidence limits; never present a preview or partial check as manufacturing
   approval.
