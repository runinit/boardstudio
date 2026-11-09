# BoardStudio

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Experienced keyboard designers working with keyboard components, layouts,
KiCad, and CAD tools. They need precise control and fast iteration without
routine tasks being buried among rarely useful settings.

## Product Purpose

BoardStudio is a visual design workbench for arranging keys and components,
deriving board and case geometry, authoring keymaps, and exporting coherent
artifacts for downstream work. Success means a designer can iterate on a layout, understand its effect on
the board and case, and carry a consistent design into KiCad, CAD tools, and firmware builds.

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
- The workbench connects Layout, PCB, Keymap, Keycaps, Case, Parts, and Export, including keymap
  and firmware handoffs. Parts provides
  compiled footprint previews and attached model previews.
- Designers continue routing and final electrical checks in KiCad, and review
  mechanical fit using the exported geometry and downstream tools. Firmware
  handoffs are completed with a full target build and hardware verification.

## Capabilities and Constraints

- Preserve stable document, part, footprint, and net identities; undo/redo;
  precise millimetre measurements; and revision-consistent previews and exports.
  Exporters must reject stale results.
- Support direct placement and parametric matrices, component libraries,
  constraints, automatic outlines and authored cutouts, net assignments, and
  plate, tray, and lid case geometry.
- Keycap overlays and mechanical assemblies are part of the design: physical
  assembly selection (shared reversible PCB or two boards for a split) is
  separate from the PCB design, and PCB thickness stays authoritative.
- Keymaps bind each current key and encoder push button. Layout edits define a
  new hardware/keymap revision; automatic keymap migration is not promised.
- ZMK firmware handoffs (targeting v0.3.0) contain editable source, pin
  assignments, and build configuration. Source generation and device-tree syntax
  checks are not a substitute for a complete firmware build or hardware
  verification.
- Keep common placement and layout controls immediately available. Disclose
  useful specialist options in named sections. Standard footprints retain
  sensible geometry defaults; raw footprint editing belongs to custom or
  imported parts. Preserve previously saved supported parameters.
- Project files use the `boardstudio/v2` format. There is no v1
  project importer or conversion path.
- The library includes three synthetic, editable layout examples. They demonstrate
  geometry only and do not claim electrical or fabrication readiness.
- Bundled Ergogen generators are trusted inputs. User-supplied JavaScript
  generators are not executed. Footprint import supports a documented subset.
- Preview availability, export validity, and fabrication readiness are distinct.
  A 3D preview does not establish clearance or fabrication readiness. Routed
  board review and mechanical-fit verification remain downstream requirements.

## Brand Commitments

The product name is BoardStudio (one word). Use the established workflow
terminology: Layout, PCB, Keymap, Keycaps, Case, Parts, Export, components, key assemblies,
matrices, outlines, keycaps, and keymaps.

## Evidence on Hand

- [The v2 overview](README.md) records capabilities, limitations, commands, and
  package boundaries.
- The retired starter keyboard (`core/examples/demo_projects/review.rs`) is historical
  evidence only; current library examples are synthetic layout-only projects.
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
