---
status: accepted
---

# A mechanical pair keeps everything linked; exceptions are explicit

A **mechanical mirror pair** exists so that one half's design is the other's, reflected.
Closure mounts, mounting holes, openings and every other mechanical feature of the
pair follow that rule.

On 2026-10-09 the user decided:

- **Linked is the default.** Every mechanical feature of a paired half is a linked
  feature, whether the user adds it, Core proposes it (for example closure mounts from
  `initialize-closures`) or a recipe generates it. A proposal made from one half's
  outline is made once, from the pair's construction source, and is mapped to the other
  half. It is not a per-half suggestion that happens to link.
- **Exceptions are individual and explicit.** A half-specific feature (a component only
  one half carries) is made local or omitted for that feature alone. An exception never
  turns a neighbouring feature, or the pair, into an independent copy.
- **Reflection is part of the link.** Linked means mirrored through the pair's mapping,
  never copied as placed.
- **Geometry that cannot reflect is a fault of the data, not of the rule.** If a linked
  feature maps outside the other half's outline, Core keeps refusing the export and says
  the feature is linked and which half authored it. The fix is to reflect the hardware
  or to make that feature an explicit exception; Core does not silently relocate it or
  downgrade the finding.
- **Shipped data follows the rule.** The included split keyboards place the controller
  bay and TRRS jack at each half's reflected position, and link their closure mounts and
  openings, unless a feature is a recorded exception.

This extends [the typed Core mechanical contract](../plans/typed-core-edits/spec.md)
and replaces the 2026-10-09 recommendation to keep initialization proposals local
([paired half closure proposals](../plans/generator-parameters/issues/06-paired-half-closure-proposals.md)).

## Amendment: the whole second half is a reflection (2026-10-09)

The first version spoke about mechanical features. The user's intent is wider: the point
of a mirrored pair is that the second half **is** the first half, reflected, in every
respect. A pair therefore covers all of these, not only mechanical features:

- the **matrices** (cells, stagger, offsets, splay), **keys** and **layouts**;
- **components** and **parts**, with their sides, poses and references;
- **outlines** and their refinements, and **assemblies**;
- **mechanical features** (mounts, openings, closure mounts, gasket supports);
- **wiring** that is structurally the same on both halves.

Rules that follow:

- **One authored half.** The pair has a construction source. Edits to the source
  propagate to the other half through the pair's mapping. Editing the other half edits the
  same authored value, as mechanical features already do.
- **Created, not recreated.** A second half is created from the first by a Core
  operation that copies and reflects everything above and records the pair. Recipes and
  the app never rebuild the second half with their own mirror arithmetic.
- **Exceptions are per item.** A component that exists on one half only, or a deliberate
  local difference, is an explicit exception on that item (local or omitted). It never
  makes its neighbours independent, and the pair never silently drifts.
- **Source data is one half.** Measured inputs that describe a single half (the shipped
  Corne, Lily58, Sweep, Chocofi, TOTEM, KLOR, Cantor and the Sofle layouts) stay one
  half; the other is derived.

The Core operation and the document representation are planned in
[native mirrored halves](../plans/typed-core-edits/issues/16-native-mirrored-halves.md)
and need their own decision on how reflection is stored (derived each time, or stored
with a link back) before building.
