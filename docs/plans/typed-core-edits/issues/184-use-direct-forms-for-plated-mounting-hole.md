# 184: Use direct forms for the plated mounting-hole generator

Status: resolved
Type: build
Blocked by: None
Related: [Footprint construction research](../../../research/2026-10-19-footprint-form-construction.md)
Decision: User approved the single-generator scope and export/live-golden test seams on 2026-10-19.

## Problem Statement

Board designers need mounting holes to retain their dimensions, plating,
placement and exports when generator internals change. The plated mounting-hole
generator still constructs text immediately parsed back into forms, although
the non-plated generator already uses shared direct construction. Maintaining
two approaches creates avoidable serialization drift; it does not justify
redesigning every footprint generator.

## Solution

Move only the plated mounting-hole generator onto the existing direct-form build
module. Keep its source ID, parameters, saved definitions, placed parts, preview
and KiCad output unchanged. Retain the non-plated generator as a comparison
control. Introduce no generic builder or physical behavior change.

## User Stories

1. As a board designer, I want the existing plated mounting-hole source ID retained, so that saved projects identify the same footprint generator.
2. As a board designer, I want the same parameters and defaults, so that definitions need no migration or re-entry.
3. As a board designer, I want circular plated holes to keep their drill/copper dimensions, so that mounting fit remains unchanged.
4. As a board designer, I want oval holes to preserve both drill/copper dimensions, so that elongated slots retain their shape.
5. As a board designer, I want omitted/zero secondary drill dimensions to keep their fallback, so that circular holes do not unexpectedly become slots.
6. As a board designer, I want equal drill dimensions to retain circular output, so that equivalent settings produce the same footprint.
7. As a board designer, I want front/back placement unchanged, so that exported layers match my accepted design.
8. As a board designer, I want translated/rotated holes to retain their position and pad angle, so that drilling stays aligned with the assembly.
9. As a board designer, I want reference text content, position, size and layer retained, so that annotations do not change during a refactor.
10. As a board designer, I want quotes, backslashes and Unicode in references exported literally, so that annotations do not become malformed forms.
11. As a board designer, I want courtyard graphics and ordering retained, so that downstream consumers receive the same geometry.
12. As a board designer, I want the current courtyard-option behavior unchanged in this refactor, so that a migration does not silently introduce a separate behavior change.
13. As a board designer, I want saved pad identities and nets to survive normalization, so that accepted definitions retain their relationships.
14. As a board designer, I want invalid kinds to keep existing refusal behavior, so that construction changes cannot bypass Core checks.
15. As a board designer, I want non-plated holes and unrelated generators unchanged, so that their calibrated geometry is preserved.
16. As a board designer, I want preview/export to keep Rust generation inside Core, so that they do not acquire divergent implementations.
17. As a maintainer, I want existing form/number construction reused, so that serialization knowledge stays local without another builder interface.
18. As a maintainer, I want missing oval and lexical edge cases characterized before the port, so that tests prove preservation rather than newly recorded expectations.

## Implementation Decisions

- Scope is one direct-form consumer: the ceoloide plated mounting-hole generator. Reuse the existing build module interface, leaving the non-plated generator unchanged. Add no generic pad, text, graphics or mounting-hole framework.
- Replace text templates and string accumulation with direct expression forms and the registry's existing direct-form body kind. Retain text support for other generators.
- Keep source ID, version, declarations, defaults, licence, attribution and Rust/Core ownership unchanged under ADR-0003 and ADR-0009.
- Keep physical calculations generator-owned: secondary-drill fallback, outline allowance, exact circle/oval comparison, courtyard offset, reference placement, shape and plating. Similar syntax does not establish shared physical defaults.
- Preserve expression order, empty pad number, wildcard copper/mask layers, coordinates, rotation, atom/string choices and number spelling. Retain concatenated-sign spelling locally where necessary; do not change the shared formatter.
- Preserve saved pad IDs and nets through existing normalization. Add no allocation, sorting or terminal rules.
- Keep model-path rewriting, legacy normalization, board-object separation and arc conversion in their existing modules. Authoring forms is distinct from normalization or preview interpretation.
- Preserve the currently ineffective courtyard toggle. Its correction is a separate decision. Add no hidden validation, coercion, sanitization or geometry fixes.
- No caller interface, crate visibility, saved schema or dependency change is intended. If the build interface needs widening, stop and justify it against two concrete callers, not hypothetical adapters.
- Depth, leverage and locality come from reusing the existing module and deleting this generator's text/parse round trip, not adding a new seam or retaining duplicate production bodies. No line-count saving is promised.

## Testing Decisions

- **Approved highest focused seam:** Core's existing KiCad artifact interface. Reuse the generator-export harness's definition, placed-part, project and export setup; add no mock registry, adapter or private-constructor suite.
- Compare outputs callers receive: serialized/parsed forms, drilling, copper layers, courtyards, literal references and refusals. Do not assert helper calls or expression-building arrangement.
- **Approved retained existing seam:** live bundled-generator goldens for exact form order/spelling, normalized geometry, saved pad identity, nets and export. Final board export alone cannot establish all these properties. Do not replace expectations after changing production code.
- Prior art: `bundled_generators_match_the_golden_baseline`, mounting-hole fixtures, Core generator-export tests and `every_bundled_generator_exports_a_board_kicad_parses`. Recorded-form-only tests do not replace live rendering.
- Before porting, add coverage for unequal nonzero secondary drill dimensions in both aspect ratios, zero/equal secondary dimensions and a zero computed courtyard endpoint. Characterize negative outline inputs and current errors. Use existing fixtures/tests, not a new baseline framework.
- Verify defaults, generator side independently of placed-part side, off-origin positions and rotations of zero, ninety and thirty-seven degrees. Retain non-plated and plated copper/drill distinctions.
- Verify the current courtyard-toggle behavior and references containing quotes, backslashes and Unicode, retaining reference effects/location without new visibility controls.
- Verify normalized pad IDs/nets and invalid-kind refusal. Keep numeric-string refusal rather than restoring JavaScript coercion.
- Run `python3 scripts/check.py quick` for changed-path footprint golden/Core export/lint/compile coverage and `python3 scripts/check.py` before closure, following the [verification workflow](../../../../README.md#checks). Run builds sequentially against the shared target; keep long output/receipts in `.evidence/`.
- Keep the real KiCad compatibility oracle. If unavailable or deliberately bypassed, report the gap rather than claiming external compatibility. Obtain the combined spec+standards closure review required for geometry-sensitive changes by the [CAD closure decisions](../../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user).

## Out of Scope

- Migrating remaining generators, including Molex connectors.
- Universal engine, data language, family builder, generic pad-bank module, adapter or test framework.
- Changed calibration, dimensions, pad order, nets, plating, formats, escaping policy or normalization.
- Fixing courtyard toggles, reference visibility, fallbacks or numeric edge behavior during the port.
- Core validation, migrations, public interfaces, dependencies or JavaScript generation.
- Promising line-count reduction or measured speed improvements without evidence.

## Further Notes

The [research](../../../research/2026-10-19-footprint-form-construction.md)
records dirty working-tree evidence at HEAD
`e326d17778524bd1704c76eb71e2c5bb7b773f10`. Direct forms already exist; repeated
syntax across mounting-hole and Molex generators does not establish a universal
construction policy. The user approved reuse for this one generator and the two
existing test seams on 2026-10-19.

There is no blocking dependency on [182](182-reuse-parsed-kicad-board-source.md)
or [183](183-align-generator-editor-null-defaults.md). This ticket owns the plated
hole port and its tests. Preserve existing edits in build/generator/export tests
and coordinate shared tests; do not merge authoring and interpretation rules.

Oval/negative numeric behavior, full lexical compatibility and external KiCad
validation of new cases remain unverified. Characterize before porting; do not
regenerate broad goldens to manufacture agreement. No implementation or test
execution is claimed by this spec.
