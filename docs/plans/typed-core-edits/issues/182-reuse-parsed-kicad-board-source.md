# 182: Reuse parsed KiCad board source across artifact projections

Status: resolved
Type: build
Blocked by: None
Related: [Board projection research](../../../research/2026-10-19-kicad-board-projection-deepening.md)
Decision: User approved the narrowed scope and existing artifact-request test seam on 2026-10-19.

## Problem Statement

When importing or previewing a KiCad board, users need its visible artwork and
physical PCB geometry to retain their established meaning. The same original
board is parsed independently for several projections, while callers coordinate
exact geometry, compatibility contours and placement. Changes to that coordination
can make the visible board and physical interpretation disagree.

The problem is duplicated source interpretation setup, not the size of the preview
function. Exact physical interpretation is intentionally stricter than preview.

## Solution

Reuse one parsed original board within each relevant artifact operation while
preserving the separate graphical and exact projections, placement rules,
diagnostics and provenance. Users see no new controls or changed project format.
Unsupported physical geometry remains refused or visibly gated, while importable
circuit and source data remain available under the current import policy.

## User Stories

1. As a board designer, I want a valid KiCad board to produce the same preview, so that an internal refactor does not alter my design's appearance.
2. As a board designer, I want copper, mask, silkscreen and saved zone fills preserved, so that I can inspect the same board details.
3. As a board designer, I want an unfilled zone to keep its explanatory diagnostic, so that I know to refill it in KiCad.
4. As a board designer, I want unsupported graphics and approximated pads to retain their diagnostics, so that approximations are not mistaken for complete interpretation.
5. As a board designer, I want diagnostic ordering and deduplication retained, so that repeated previews remain understandable and deterministic.
6. As a board designer, I want escaped drawing text preserved, so that labels remain consistent with source intent.
7. As a board designer, I want source-footprint fabrication artwork and board-preview filtering to remain distinct, so that the refactor does not unexpectedly add or hide layers.
8. As a board designer, I want model paths, references, identities, side and transforms preserved, so that the same models remain correctly located.
9. As a board designer, I want hidden model behavior preserved, so that models I excluded do not reappear.
10. As a board designer, I want preview revision identity preserved, so that a result identifies the design revision it represents.
11. As a board designer, I want display thickness defaults distinguished from authored thickness, so that an unknown physical dimension is not silently invented.
12. As a mechanical designer, I want authored lines, arcs and circles retained exactly, so that preview tessellation never substitutes for physical PCB geometry.
13. As a mechanical designer, I want contour identities and nearest hole ownership preserved, so that nested islands and cutouts remain the same physical regions.
14. As a mechanical designer, I want open, ambiguous, touching or unsupported geometry to retain its named refusal, so that incomplete physical geometry cannot pass unnoticed.
15. As a mechanical designer, I want footprint-local pad positions and board-frame pad angles preserved on either side, so that holes and slots remain correctly located.
16. As a mechanical designer, I want drill offsets, dimensions, source IDs and source placement/flip preserved, so that the physical board instance matches the imported board.
17. As a mechanical designer, I want only supported resolved through vias treated as complete through-holes, so that blind or unknown-depth vias do not become fabricated cuts.
18. As a module importer, I want a physically unsupported source to retain its circuit and provenance with existing mechanical gates, so that I can qualify it without losing usable data.
19. As a module importer, I want compatibility contours, holes, constituent placement and exact geometry to keep the same frame, so that importing an existing module does not shift it.
20. As a board designer, I want malformed or oversized preview input to retain bounded failure behavior, so that unsafe input is not silently accepted.
21. As a maintainer, I want original-board parsing coordinated once inside Core's artifact module, so that callers do not recreate source setup for each projection.
22. As a maintainer, I want existing artifact operations to verify these behaviors, so that no test-only parsing interface or replacement framework is needed.

## Implementation Decisions

- Keep the change private to Core's artifact interpretation modules. Preserve existing artifact request/reply contracts, accepted document behavior and persisted formats.
- Concentrate original-board parsing, root selection and source lifetime ownership in the smallest private parsed-board module necessary for reuse. Preview and exact extraction consume that root; module import reuses the root already needed for circuit interpretation.
- Retain original text alongside parsed spans without a second normalized board representation. Introduce no trait, adapter registry, generic visitor, cache or configurable framework.
- Remove superseded repeated original-source parsing/root selection. A pass-through wrapper retaining all existing parsing fails the deletion test. The depth, locality and leverage come from removing caller coordination, not merely relocating the preview function.
- Preserve operation-specific error categories, relevant diagnostics and validation order, including placement refusal before source interpretation and preview byte-limit checks before expensive raw-source preview parsing. Do not impose a new whole-import limit.
- Retain graphical and exact projections over the same parsed input; do not force one traversal or derive exact geometry from preview polygons.
- Preserve preview sampling, endpoint joining, ordering, layer filtering, text decoding, pad approximations, zones, model paths and diagnostic sorting/deduplication.
- Preserve exact extraction's authored curves, source identities, contour ownership, nested-inventory refusal, drill classification, via-depth refusal and feature ordering.
- Preserve KiCad Y conversion, footprint-local anchors, absolute pad angles, drill-offset rotation, side behavior and external placement/flip. Leave the existing arithmetic and differently validated numeric parsers intact.
- Preserve optional authored thickness separately from display defaults and retain module import's distinct thickness metadata/gating policy.
- Preserve module import's compatibility tessellation and established center as the shared frame. Its synthetic footprint is a distinct input, not another original-board parse to eliminate.
- Keep strict direct physical reads and gracefully gated module imports distinct. Do not introduce a global strictness setting.
- Footprint generation and generator parameter checks remain in Core under ADR-0003 and ADR-0009. No generator rendering change is authorized.

## Testing Decisions

- **Approved single primary seam:** the existing stateless artifact request interface, exercising board preview, direct board-source read and module-board import. No test-only AST interface is needed.
- Test observable results and errors, not AST layout, helper names, traversal count or parser-call counters. Review the diff separately to verify parse reuse and deletion of superseded paths.
- Use deterministic small fixtures with analytical coordinate/radius expectations and complete relevant output records.
- Prior art: `projects_copper_holes_models_and_saved_fills`, `decodes_escaped_text_whitespace_in_preview`, `footprint_projection_includes_fab_without_changing_board_preview_layers`, `rejects_open_edges_and_non_finite_values`, and `joins_edges_and_classifies_cutouts`. Retain these or promote their assertions rather than duplicating them.
- Artifact-level prior art includes `through_vias_keep_uuid_centres_and_diameters_with_slots_and_source_placement`, `unsupported_via_depth_gates_module_exact_geometry_and_retains_source_provenance`, `direct_board_source_preserves_a_circular_outline_hole_without_refitting`, `direct_board_source_preserves_nested_islands_and_nearest_hole_owners`, and `direct_board_source_keeps_unknown_thickness_and_refuses_incomplete_outlines`.
- Cover mixed artwork/pads/vias/models, explicit and absent saved fills, hidden models, reference fallbacks, literal paths, nondefault model transforms, escaped labels, approximation diagnostics, revision identity and layer filtering.
- Cover missing/explicit thickness, malformed/empty/wrong-root source, invalid/nonfinite dimensions and preview size limits. Combine invalid placement with malformed source to protect refusal order. Preserve each operation's existing error envelope.
- Cover front/back footprints with independent nonzero footprint/pad angles and drill offsets, round drills, oval slots and external translation/rotation/flip.
- Reuse legacy/modern identity, arc, circle, nested island/hole, open/ambiguous contour and contour-budget regressions. Do not make preview tolerance exact-source acceptance policy.
- Cover supported and unsupported via depths across all three operations: preserve preview behavior, direct-read refusal and gated imports retaining provenance/circuit data without fabricated exact drills.
- Retain source-footprint artwork and generated-board preview integration coverage at their existing interfaces.
- Collect baseline/post-change Core native evidence, then run `python3 scripts/check.py quick` for the implementation's changed paths and `python3 scripts/check.py` before closure, following the [verification workflow](../../../../README.md#checks). Run builds sequentially against the shared target and keep long output/receipts in `.evidence/`.
- Obtain one combined spec+standards closure review for this geometry-sensitive refactor under the [CAD closure decisions](../../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user). No benchmark threshold or speed claim is required.

## Out of Scope

- New KiCad syntax, geometry fixes, changed tolerances, diagnostic cleanup or stricter global import policy.
- Replacing exact curves with sampled polygons, changing CAD allowances or backend work.
- Replacing compatibility contours or changing module centering.
- New UI, worker transport, project migration, public parsed-board contract, dependency or performance framework.
- Generator helper consolidation or parameter policy changes.

## Further Notes

The [research](../../../research/2026-10-19-kicad-board-projection-deepening.md)
records source evidence at HEAD `e326d17778524bd1704c76eb71e2c5bb7b773f10` and the
dirty working tree. Preserve existing user edits. The initial three-direct-caller
claim was incorrect: preview board-source has two direct callers; its wrapper has
three production consumers. Exact extraction already owns substantial geometry.

The user approved this narrower scope and test seam on 2026-10-19. There is no
blocking dependency on [183](183-align-generator-editor-null-defaults.md) or
[184](184-use-direct-forms-for-plated-mounting-hole.md). This ticket owns private
board parsing/projection coordination; coordinate shared artifact tests without
moving generator checks or authoring rules into it.

Parser ownership/lifetime details, diff size and performance benefit remain
unknown. If repeated parsing cannot be removed without widening public contracts
or changing outcomes, report the blocker rather than building a framework.
Research was static: no build, test or benchmark execution is claimed.
