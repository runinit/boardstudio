# 14: Decide how starters and demos are rebuilt

Status: resolved
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md), [07](07-matrix-anchors.md), [08](08-uniform-pitch-consumers.md), [09](09-inspector-and-toolbar-prototype.md), [11](11-mirrored-halves-rules.md), [12](12-relations-and-highlight.md), [13](13-format-reset-and-recovery.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

Graduated from the map's Starters fog now that layout, anchors, editing and explanation
semantics are settled. ADR-0034 requires rebuilding starters and fixtures under the new
format rather than migrating them. Decide:

- which current bundled examples, starter/demo paths and fixture families must be rebuilt;
- which preserve their demonstrated geometry and which intentionally demonstrate Spread,
  Padding, Shift/Bend, independent rotations or matrix anchors;
- whether any split example's thumb cluster becomes a separate anchored matrix and which
  linked-half/exception behaviour it demonstrates;
- default dimensions, pitch and parameter choices, and the expected visible layout;
- the golden/regression checks, provenance and attribution needed for replacement data.

First inventory the actual current examples and fixtures rather than relying on retired
starter names. Keep geometry-only examples labelled as such; do not introduce electrical
or fabrication-readiness claims. This ticket chooses replacement data and acceptance,
not the implementation or a new hardware recipe.

## Answer

Decided with the user on 2026-10-23 in two rounds; all six recommendations were
accepted. These are replacement-data requirements for the planned version-7
implementation, not claims that the current loose-part examples already have matrices.

### Scope and authoring

Keep the three existing entries and their identities in
`web/crates/library/src/demo_projects.rs::DEMO_PROJECTS`: **Split 42**, **Studio 60**
and **Macro 12**. Keep their editable, synthetic **geometry-only** role and fabrication
disclaimer. Do not revive retired hardware starters or add a second example collection.

Rebuild `DemoProject::document` from clean, named layout parameters and Core-generated
matrix members, not baked authoritative Part poses or hidden legacy compensation.
Keep the synthetic Demo switch, 18×18 mm keycaps and one-board example scope. All
example matrices use pitch **(19.05, 19.05) mm** and edge gap **1.05 mm**, giving
18×18 mm pitch-derived key bounds. This is example data, not a change to global setup
defaults. Preserve unrelated board/material/outline intent and update ownership and
outline references to the generated members.

Open each example as a fresh project copy through the existing library/runtime path.
The ordinary blank New project path stays blank; it does not silently open a demo.
New template members have matrix-coordinate identities; do not rewrite saved copies
or preserve the old loose `key-NN` IDs by bypassing matrix membership. References,
Layout membership and counterpart mappings must use the new generated identities.
No controllers, wiring, physical-board pair, hardware profiles or fabrication-ready
recipe are added. Ordinary Core-generated logical inputs, if any, remain unassigned.

### Studio 60 and Macro 12

Each owns one Layout containing one unanchored matrix. Origins below are world
coordinates; row/column labels in this answer are one-based.

| Example | Rows × columns | Origin (mm) | Nondefault carried values |
| --- | --- | --- | --- |
| Studio 60 | 5×12 | (−104.775, −38.1) | Row 2 Shift X = 4.7625 mm only. |
| Macro 12 | 3×4 | (−28.575, −19.05) | Row 2 Shift X = 4.7625 mm only. |

All other transforms are defaults: absent Spread/Padding mean pitch, no local mirror,
and zero rotations, offsets, other Shift and Bend. Row 2's Shift carries once to
every later row; it is not another Shift on every row. This reproduces the existing
60/12 key positions and zero headings exactly within BoardStudio's numeric tolerance.

### Split 42: linked main and anchored thumb groups

Keep the existing single board. Add **two linked Layouts**, with the left authored
Layout as source and the right derived through the same-board reflection about X = 0.
Each Layout owns a 3×6 main matrix and a 1×3 thumb matrix, for **42 keys total**.
There are no default Local/Omitted exceptions. Create/maintain the counterpart group
through Core's linked-layout rules, not a second hand-authored right-side formula.

The following source recipe expresses the accepted coordinates without compensation:

| Left source matrix | Frame/anchor | Local layout |
| --- | --- | --- |
| Main, 3×6 | World Origin (−31, −19.05), Rotation 0°, local X mirror. | Row 2 and Row 3 each have Shift X = −2.5 mm; all other steps/turns default. |
| Thumb, 1×3 | Anchor to Main Row 3, Column 3 (zero-based `(2,2)`), Follow X/Y/Rotation; relative Origin (0.1, 20.95), Rotation 0°, local X mirror. | Column 2 and Column 3 each have Stagger = +3 mm; other steps/turns default. |

The anchor key is at (−64.1, 19.05) mm with heading 0°. The thumb Origin therefore
puts its first key at (−64, 40), with successive keys at (−83.05, 43) and
(−102.1, 46). Its relative Origin is an authored anchor offset, not world-pose baking.
The source's local X mirror makes the negative row Shift reproduce the current
left main group's positive world-X progression. Key headings remain 0°.

The right main and thumb matrices follow ticket 11's reflection/anchor rules. In
particular, the corresponding thumb anchor is its right Main Row 3, Column 3; the
relative Origin X reflects to −0.1 mm, while Y remains 20.95 mm. All 42 keycaps keep
their original sizes and all headings remain 0°.

**Approved geometry change:** keep the left-side and both thumb-cluster poses.
Correct the right main group to the exact reflection of the left. Compared with the
current loose-part example, its Row 1 X coordinates are unchanged, Row 2 changes by
**−5 mm**, and Row 3 by **−10 mm**; Y coordinates are unchanged. Do not claim all old
Split 42 poses were preserved. Regenerate its dependent outline/preview expectations
deliberately from the new member poses.

### Parameter coverage and fixture rebuilding

Do not add curvature or extra spacing changes just to exercise every field in the
library. Nondefault Spread, Padding, Bend and independent rotations, plus exhaustive
Local/Omitted, partial Follow, deletion, inverse-edit and refusal cases, belong in
focused fixtures. Preserve the three examples' clarity.

The source of truth for the demos remains their authored builder, not a new committed
JSON/archive dump. Rebuild affected positive schema/layout inputs for version 7; do not
bulk-stamp old data or run it through a compatibility migration. Intentional legacy
inputs remain old for ticket 13's refusal tests.

- Replace `core/tests/fixtures/matrix-projections.json`'s old TypeScript-captured math
  oracle with the regenerated BoardStudio-only extended-feature regressions already
  settled in tickets 03/04. Keep their feature intent, not obsolete numeric values.
- Regenerate affected documents, scene payloads and expected poses in the boundary
  transcript and related current-schema fixtures. Review `synthetic-outline.json`
  and other serialized inputs for actual schema/pose dependencies; preserve their
  outline, identity, changed-ID, history or other contract assertions.
- Adapt affected matrix/companion/mirror/anchor/scene and mounted UI fixtures without
  weakening the behaviour being tested. Unrelated CAD, footprint and hardware goldens
  are not swept into the change without a demonstrated dependency.
- Keep pinned Ergogen config, commit, name-to-coordinate mapping and licence notices
  from ticket 03. Ergogen parity compares within **1e-6**; BoardStudio-only regressions
  use **1e-9** as settled in ticket 04. Ordinary tests need no upstream runtime.
- Preserve provenance: the examples are authored synthetic coordinates, not measured
  real-keyboard designs. Replacement expectations identify their authored scenario or
  pinned upstream source; no new hardware/attribution claims are invented.

### Required implementation acceptance

Use the rebuilt library documents and actual Core/Session paths to verify:

- Exactly the same three entries, geometry-only labels and fresh-copy workflow;
  60/12/42 keys, 18×18 mm sizes, 19.05 mm pitch and no new hardware/readiness claims.
- Valid version-7 documents, complete matrix/board/Layout/outline membership and stable
  coordinate/counterpart identities. Two copies do not share mutable project state.
- Studio 60/Macro 12 retain their poses/headings, with only Row 2's carried Shift needed.
- Split 42 has two linked Layouts, four matrices and the specified same-Layout anchors.
  Left and thumb poses are retained, right main poses change only as approved, and
  all headings remain zero.
- Editing a main anchor key moves its thumb matrix; editing either linked half routes
  through Core to its counterpart. Scene parts, previews and generated geometry agree;
  preview/cancel and Undo/Redo preserve the relevant contracts.
- Fixture regeneration separates new geometry expectations from unchanged identity,
  companion, constraint, scene and persistence invariants, with focused edge-case
  coverage rather than extra default example complexity.

Verification was source/test inventory plus a numeric check of the coordinate recipes
against the current demo formulas (`.evidence/ergogen-layout-parity-14/check-recipes.py`
and `recipes.log`, gitignored). No example code, fixtures, builds or application runtime
tests were changed/run. Those checks do not establish production support for multi-matrix
Layouts or anchors; implementation must supply the acceptance above.

## Comments

### 2026-10-23 — current example and fixture inventory

- The only active bundled list is `web/crates/library/src/demo_projects.rs::DEMO_PROJECTS`:
  **Split 42**, **Studio 60** and **Macro 12**, built by `DemoProject::document` and
  opened as fresh copies through `Runtime::open_demo_copy`.
- These currently contain independent Part poses, not matrices or Layouts. All use
  the synthetic Demo switch, 18×18 mm keycaps, no electrical inputs/hardware profiles,
  one board and a part-envelope outline. They are geometry-only examples.
- Studio 60 is 5×12 and Macro 12 is 3×4 at 19.05 mm pitch. Rows after row 0 share a
  single +4.7625 mm X displacement; it is not another +4.7625 mm on every row.
- Split 42 has two 3×6 main groups plus three thumb keys per side, at 19.05 mm steps.
  Its main positions are `x = side*(31 + column*19.05) + row*2.5`,
  `y = (row−1)*19.05`; thumbs are `x = side*(64 + thumb*19.05)`,
  `y = 40 + thumb*3`. Both main groups shift in the same +X direction, so they are
  not exact reflected counterparts. Its visual halves are not linked and its thumbs
  are not anchored matrices. A real linked conversion therefore needs an explicit
  geometry choice, not a claim that all existing poses remain identical.
- Demo builders are the source of truth; no committed generated demo-project JSON or
  archive was found. `core/examples/demo_projects/review.rs` is absent and mentioned
  only as historical evidence in PRODUCT.md.
- Relevant serialized expectations include the boundary transcript, synthetic outline
  input and old matrix-projection oracle, plus matrix/companion/mirror/scene tests.
  Rebuild affected current-schema/layout fixtures while preserving their intended
  contracts. Tickets 03/04 already replace the old TypeScript pose oracle, with `1e-6`
  Ergogen tolerance and `1e-9` BoardStudio-only regressions. Unrelated CAD/footprint
  fixtures are not automatically in scope.

This is a source/test inventory with focused inspection of the demo builder, not a
passing test run. No examples, fixtures, builds or application runtime were changed.

### 2026-10-23 — round 1 accepted

The user answered “Y to all” to these three recommendations:

1. Refresh the existing three library entries for version 7, preserving their example
   identities and geometry-only labels. Do not add a second collection or revive retired
   hardware starters.
2. Preserve each example's intended shape, key count, pitch and key sizes unless an
   explicit demonstration change is agreed. Author the new named layout parameters,
   not baked part poses or hidden legacy compensation. Old-engine coordinates are not
   retained merely because they were captured previously.
3. Keep examples understandable and put exhaustive Local/Omitted, partial Follow,
   deletion, inverse-edit and refusal coverage in focused regression fixtures instead
   of turning each library entry into a stress test.

### 2026-10-23 — round 2 accepted

The user answered “Y to all” to these three recommendations:

4. Studio 60/Macro 12 become single-matrix 5×12/3×4 Layouts with pitch 19.05 mm,
   edge gap 1.05 mm, 18×18 mm keycaps and Row 2 Shift X = 4.7625 mm only. Other
   transforms default; preserve current poses/headings. Use matrix-member identities
   in new copies without rewriting saved projects.
5. Split 42 becomes two linked Layouts on the existing board, each with 3×6 Main and
   anchored 1×3 Thumb matrices. Keep the left as source, preserving left/thumb poses.
   Make the right exact reflection: main Row 1 unchanged, Row 2 X −5 mm, Row 3 X
   −10 mm. Keep sizes/pitch, with no new controller/wiring/physical-board pair.
6. Do not add curvature/spacing changes to show every field; use focused fixtures for
   nondefault Spread/Padding/Bend and independent rotations instead.

All branches are settled. The ticket is resolved as a replacement-data decision, not
an implementation claim.
