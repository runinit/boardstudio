# Shared footprint form construction — research and bounded recommendation

Researched 2026-10-19 against HEAD `e326d17778524bd1704c76eb71e2c5bb7b773f10`
(2026-10-19, “Resolve typed Core tickets 141 and 175 after full verification”) and
the **current, dirty working tree**, not HEAD alone. Source links below refer to
working-tree files, with symbol names and observed line ranges. They are not frozen
copies: recheck after concurrent work lands. No implementation, staging, commits,
builds, Cargo invocations or test execution were performed.

## Verdict

**Proposed decision:** do not introduce a shared generic pad/text/graphics builder.
The strongest small follow-up is to reuse the existing direct-form build module
for **one additional generator, `ceoloide/mounting_hole_plated`**, following the
already migrated `ceoloide/mounting_hole_npth`. Leave generator-specific geometry
and export normalization where they are. This is a limited continuation of an
existing migration, not evidence for a new footprint framework.

**Evidence:** the existing build module already centralizes expression assembly,
JavaScript-compatible number spelling, placement, sided layer names and font
effects; the non-plated mounting-hole generator consumes it directly. The plated
generator still constructs the same kinds of forms as text and the registry
parses that text back. See [`build::{form,num,at,placement,layer,effects}`,
lines 1–54](../../footprints/src/build.rs#L1-L54),
[`mounting_hole_npth::body`, lines 38–69](../../footprints/src/generators/ceoloide/mounting_hole_npth.rs#L38-L69),
[`mounting_hole_plated::body`, lines 38–99](../../footprints/src/generators/ceoloide/mounting_hole_plated.rs#L38-L99)
and [`Registry::render`, lines 244–249](../../footprints/src/registry.rs#L244-L249).

**Inference:** eliminating this generator's text-then-parse round trip improves
locality of serialization knowledge without a new interface. A measurable speedup
or net line deletion has **not** been established; neither is an acceptance claim.

## Constraints and history

- **Evidence:** ADR-0003 requires footprint generation in Rust inside Core, with
  source licences retained. No JavaScript adapter or alternate execution path is
  justified. [ADR-0003, lines 5–20](../adr/0003-rust-footprint-generators-in-core.md#L5-L20).
- **Evidence:** ADR-0009 puts generator parameter name/type checks in Core and
  shares the declared-kind rule with render. This work must not add a second
  schema or coerce numeric text back into numbers. [ADR-0009, lines 14–39](../adr/0009-generator-parameters-checked-in-core.md#L14-L39).
- **Evidence:** direct forms are already supported beside legacy text by
  [`Body`, lines 16–22](../../footprints/src/registry.rs#L16-L22).
  Commit [2fe19cf42, 2026-10-19](https://github.com/runinit/boardstudio/commit/2fe19cf42)
  introduced that choice, the build module and the non-plated mounting-hole port.
  [0ea5b26d5, 2026-10-19](https://github.com/runinit/boardstudio/commit/0ea5b26d5)
  subsequently addressed literal generator text. A claim that every generator
  must emit text, or that escaping is wholly uncentralized, is outdated.
- **Evidence:** the earlier universal-engine research recommends direct expression
  trees but also discusses a much larger program representation. It is research,
  not authorization for this candidate to replace Rust generators with a language.
  [Earlier recommendations, lines 139–158](universal-parametric-footprint-engine.md#L139-L158).
- **Evidence:** glossary usage is “footprint generator,” stable source ID and
  typed “generator parameters.” [Glossary](../../GLOSSARY.md#language).

## Repeated conventions established across concrete generators

### Small, immediately reusable pair: the two ceoloide mounting holes

| Construction convention | Non-plated generator | Plated generator | Classification |
|---|---|---|---|
| Sided copper layer and part placement | `layer` and `placement` | interpolated side and `p.at()` | Shared representation/placement convention; reuse existing build functions. |
| Reference property, sided silk, 1 mm font and 0.15 mm thickness | local Y 2.55 | local Y 3 | Repeated effects, but reference position remains generator-owned. |
| Empty-number pad at local origin, absolute part rotation, wildcard copper/mask layers | `np_thru_hole circle` | `thru_hole circle` or `oval` | Shared form grammar, not interchangeable plating/shape policy. |
| Size/drill relationship | independent `hole_size` and `hole_drill` | copper size is drill plus twice outline | Legitimate different physical meaning; do not merge parameters. |
| Courtyard graphics | none | circles or ordered lines on both sides | Legitimate difference; do not create a universal courtyard default. |

**Evidence for the table:** [`mounting_hole_npth::{PARAMS,body}`, lines 17–69](../../footprints/src/generators/ceoloide/mounting_hole_npth.rs#L17-L69)
and [`mounting_hole_plated::{PARAMS,body}`, lines 15–99](../../footprints/src/generators/ceoloide/mounting_hole_plated.rs#L15-L99).
The plated body calls `ref_hide`, but that accessor currently returns the empty
string; there is no active hide feature to add or normalize during the port.
[`RenderContext::ref_hide`, lines 164–167](../../footprints/src/context.rs#L164-L167).

### Broader pair investigated, not selected: two infused-kim Molex connectors

**Evidence:** both 2-pin and 5-pin generators emit front/back SMD roundrect pads
with Cu/Paste/Mask layers and a 0.25 roundrect ratio, use 1/0.15 reference effects,
and repeat silk/fabrication/courtyard line grammar. However the 2-pin front pad
order is MP, MP, 2, 1, while the 5-pin order is 1, MP, MP, 2, 4, 5, 3. Back-side
pad ordering differs again. Reference positions and outlines differ too.
[`conn_molex_pico_ezmate_1x02::body`, lines 63–138](../../footprints/src/generators/infused_kim/conn_molex_pico_ezmate_1x02.rs#L63-L138);
[`conn_molex_pico_ezmate_1x05::body`, lines 66–159](../../footprints/src/generators/infused_kim/conn_molex_pico_ezmate_1x05.rs#L66-L159).

**Inference:** there is real repeated syntax, but matching a few widths and ratios
does not establish a universal hardware rule. A generic connector/pad-bank module
would need callers to specify nearly all the important ordering, dimensions,
layers, angles and nets. That is a shallow interface, and migrating both complete
generators solely to exercise it is not the smallest complete change. Retain them
unchanged in this slice, including their licences and calibrated geometry.

## Real callers and ownership

1. **Evidence:** `Registry::render` resolves declared generator parameters,
   definition/part overrides, defaults and nets before invoking either body kind;
   net parameters allocate in declaration order. Preserve the public interface and
   both body kinds. [`Registry::render`, lines 157–249](../../footprints/src/registry.rs#L157-L249).
2. **Evidence:** definition normalization and catalogue creation render through
   that registry. Normalization preserves saved pad IDs/nets **by index**, so
   reordering pads is a document compatibility change, not harmless formatting.
   [`Registry::normalize`, lines 82–110](../../footprints/src/definition.rs#L82-L110);
   [`Registry::{catalogue,catalogue_entry}`, lines 212–231](../../footprints/src/definition.rs#L212-L231).
3. **Evidence:** Core's `render_jobs` calls `bundled().render`, obtains model
   bindings, resolves model paths, invokes `export_forms`, then upgrades legacy
   arcs. Preview and export both use these jobs. [`generation::{render_jobs,preview,export}`,
   lines 21–74](../../core/src/artifact/kicad/generation.rs#L21-L74).
4. **Evidence:** export normalization rewrites model paths, converts legacy
   `module` heads to `footprint`, and separates board objects. It is not another
   pad/text constructor. [`export_forms`, lines 65–93](../../footprints/src/export.rs#L65-L93).

**Proposed scope inventory:** change the plated generator body and its body-kind
selection; add focused tests/fixture cases only where existing coverage is absent.
Reuse build functions as they stand; change the build module only if an actual
small omission is demonstrated during implementation, not to introduce a generic
pad/line/reference interface. No caller, export normalization, parameter schema,
catalogue source ID, crate export or saved-document change is expected. The
non-plated generator is a regression control, not another rewrite target.

## Deletion test and alternatives

- **Existing build module:** deleting its implementation would distribute number
  spelling, placement and form assembly back to direct-form callers. Reusing it
  gives leverage without inventing a seam. [Implementation, lines 1–54](../../footprints/src/build.rs#L1-L54).
- **Proposed generic pad/line wrapper:** no established additional policy remains
  after coordinates, shapes, layers, rotations, ordering and optional clauses stay
  with each generator. Deleting such a wrapper would largely remove plumbing.
  **Reject** until a second caller demonstrates a missing shared invariant.
- **Chosen slice:** delete the plated generator's format templates and its local
  string accumulation/number-to-text scaffolding, replacing them with existing
  expression construction. Do not retain a duplicate production text body as an
  adapter, add a migration framework, or remove the legacy text body kind globally.
  **Unknown:** expression assembly may be longer than the original templates.
  This is justified only as a bounded removal of the render/parse step, not “bloat
  reduction” measured by lines.

## Test prior art and proposed seams — confirmation required

**Proposed focused-test seam:** Core's existing KiCad artifact interface
(`kicad::export`, with preview regression through the same artifact module when
needed). It is the highest existing seam exposing what the user receives. Reuse
the current generator-export test setup rather than adding an adapter, mock
registry, private helper tests or a new fixture runner. [`generator_export::{definition,part,project,request,export_board}`,
lines 12–86](../../core/tests/generator_export.rs#L12-L86).

**Existing lower-level regression coverage is retained, not replaced:**

- The live generator golden runner compares serialized rendered forms, net
  snapshots, geometry, model bindings, export and normalized definitions.
  [`check_case`, lines 759–933](../../footprints/tests/generators.rs#L759-L933).
  This catches form spelling/order and saved-pad identity that a Core board
  export alone cannot establish. It exercises the production registry interface,
  not private build-function structure.
- The plated fixture has 14 cases: defaults, `include_courtyard` false, pose/side
  combinations and legacy numeric text. It has no nonzero `drill_y` case.
  [Plated fixture, lines 1–16](../../footprints/tests/golden/generators/ceoloide__mounting_hole_plated.json#L1-L16).
  The non-plated fixture has 16 cases and is an unchanged comparison control.
  [Non-plated fixture](../../footprints/tests/golden/generators/ceoloide__mounting_hole_npth.json#L1-L18).
- `golden_forms` operates on **recorded forms**, so it is supporting parser/export
  prior art, not by itself proof that a changed body still emits those forms.
  [`recorded_cases`, lines 22–41](../../footprints/tests/golden_forms.rs#L22-L41).
- Existing Core coverage exports every bundled generator to a KiCad-parsed board;
  missing KiCad 10 fails unless deliberately bypassed. A bypass must be reported,
  not presented as external compatibility evidence. [`kicad_parses`, lines 114–120](../../core/tests/generator_export.rs#L114-L120);
  [`every_bundled_generator_exports_a_board_kicad_parses`, lines 252–298](../../core/tests/generator_export.rs#L252-L298).

**Proposed behavior-preservation scenarios:**

1. Default circular plated hole and unchanged non-plated control: empty pad number,
   pad identity, drilling, copper/mask layers, plating and reference position.
2. Plated oval hole with nonzero unequal `drill_y`, including the reverse aspect
   ratio; preserve computed pad size, oval drill, both courtyard line sequences,
   widths and normalized extent. Characterize the old body before changing it;
   existing plated goldens do not cover this branch.
3. `drill_y` omitted, zero, and explicitly equal to `drill`: preserve fallback and
   the exact `size_x == size_y` branch decision. Do not substitute an epsilon.
4. Front/back generator parameters, independently front/back placed parts,
   off-origin positions, and rotations 0, 90 and 37 degrees: no extra mirroring or
   change to absolute pad angles or the downward-Y serialization convention.
5. Quoted/backslash/Unicode references survive export literally; no unrequested
   font, visibility or reference-position changes.
6. `include_courtyard` false currently still emits courtyard graphics. Preserve
   that behavior in this refactor; a fix requires a separate user-visible decision.
7. Wrong parameter kinds keep current refusal behavior. Numeric-string golden
   inputs are already explicitly treated as refusals by the live runner, not as
   instructions to preserve old JavaScript coercion.
8. Preserve number spelling and atom/string choices, including concatenated-minus
   corner cases. With a zero computed courtyard endpoint, the old template can
   spell `-0`; `num(-0.0)` instead spells `0`. Characterize zero/negative outline
   cases before porting; do not silently sanitize values or change failure modes.

Evidence for these scenarios is the two mounting-hole bodies cited above, the
live golden runner's numeric-string exception
([lines 773–795](../../footprints/tests/generators.rs#L773-L795)), and
[`js_number`, lines 8–17](../../footprints/src/number.rs#L8-L17).
These are proposed acceptance scenarios, **not tests executed during research**.

## Risks, unknowns and coordination

- **Evidence:** `include_courtyard` is declared but never read in the plated
  body; its false fixture still contains both courtyards. Do not fix it by accident.
  [Declaration/body, lines 15–99](../../footprints/src/generators/ceoloide/mounting_hole_plated.rs#L15-L99);
  [false fixture, line 4](../../footprints/tests/golden/generators/ceoloide__mounting_hole_plated.json#L4).
- **Inference:** changing expression number construction can change lexical edge
  behavior even when ordinary geometry matches. Preserve it locally with explicit
  atoms where necessary; do not weaken the shared number formatter to accommodate
  one legacy template.
- **Unknown:** no runtime baseline, KiCad validation, negative-value behavior
  experiment, oval characterization run or mutation check was performed. Before
  implementation, capture missing cases against the current body in tests; do not
  regenerate all goldens after the port to manufacture agreement.
- **Coordination:** this candidate owns only the bounded plated-hole migration and
  its verification. It depends on no sibling architecture candidate. If a sibling
  owns preview interpretation, do not move pad/graphic interpretation into build
  helpers: authoring forms and interpreting forms are different responsibilities.
  The shared build module, several generators, escaping tests and Core
  generator-export tests already contain user edits; reread and coordinate before
  later edits. The research/spec draft changes none of them.
- **Publication follow-up:** on 2026-10-19 the user approved the single-generator
  scope and export/live-golden test seams. Published as
  [ticket 184](../plans/typed-core-edits/issues/184-use-direct-forms-for-plated-mounting-hole.md),
  `ready-for-agent`. Earlier proposal language records the research stage, not an
  outstanding approval blocker.

## Checks actually performed

Read current source through CodeGraph, repository docs and ADRs; inspected Git
status, HEAD/date and footprint-related commit history; inspected current diffs
for build/non-plated/battery generator files; parsed the two mounting-hole JSON
fixtures to inventory case counts and IDs. Static document/link/section checks
were performed after writing. No Cargo, full tests, browser, external-library
documentation, benchmark or implementation verification was run.
