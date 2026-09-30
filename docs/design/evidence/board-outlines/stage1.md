# Board outlines: Stage 1 evidence and walkthrough

Updated 2026-09-30. Stage 1 is implemented in the working checkout based on
`6abf3eb951b1e9d8fc82ee96b0476730729c8a69`. Changes are uncommitted. Human
acceptance and physical fabrication are unperformed; Stages 2–4 remain pending.

## Delivered behavior

- Generated repairs the four annotated thumb/controller/reset recesses in the
  retained 85-part REVIUNG project, while preserving the wide centre valley,
  shallow optional stagger alternatives and authored cutouts.
- Cleanup applies to old and new generated outlines. Maximum gap span defaults
  to 20 mm, minimum connection width to 2 mm, edge clearance to 0 mm, and bridge
  width retains its 10 mm default. Mouth span and connection width are separate.
  Deep recess classification uses depth/span ≥ 0.25, calibrated against the
  annotated examples; it is not a universal manufacturing limit.
- Keep gap follows source components, survives reopening and remains removable
  when source movement changes a generated candidate's ID. Missing sources retain
  the last valid perimeter, display a located finding and block affected exports.
- Each PCB has one Outline object with Generated and named fixed versions.
  Bridges also appear in their associated matrix contexts and select the same
  geometry. Legacy authored source connections keep their existing attachments.
- Opening/cancelling the point editor does not save a version. The first
  committed perimeter edit or addition/cutout creates and activates a fixed copy
  in one Undo step. Copy, rename, switch, delete, Undo/Redo and reopening preserve
  independent geometry; deleting an active custom version selects Generated.
- Basic point move/insert/remove uses the existing mm grid, geometry snapping
  and Alt bypass. Exact typed coordinates retain their entered values.
- Fixed geometry stays fixed when components move. Located support, clearance,
  connectivity and width findings block affected outline/PCB/plate/case exports;
  project saving and editing remain available. Inactive versions and unrelated
  boards do not block valid output.
- Case-derived closure-clearance NPTH cutters remain openings rather than
  mounted component bodies. Ordinary excluded components and ordinary mounting
  holes still require pad/drill support; body-overhang permission does not waive
  it. This distinction preserves existing case export workflows.

## Retained artifacts

The [original project](reviung41-original.boardstudio), [before SVG](reviung41-before.svg)
and [before DXF](reviung41-before.dxf) remain unchanged. See [baseline provenance](README.md).

- [Repaired Generated SVG](reviung41-after.svg) and [DXF](reviung41-after.dxf):
  downloaded through the browser UI; every exported coordinate matches the
  visible active contour. DXF remains a closed millimetre `LWPOLYLINE`.
- [Fixed version SVG](reviung41-fixed.svg): downloaded with Fixed review active;
  every coordinate matches that fixed version rather than Generated.
- [Invalid fixed example](reviung41-fixed-invalid-example.boardstudio): a saved
  browser-test project with two fixed versions and U1 moved to X 310 mm. It
  deliberately demonstrates located blockers, saving/reopening and switching
  back to valid Generated output. It is separate from the original project.
- [Generated screenshot](reviung41-generated-light.png), [located missing support](reviung41-fixed-findings.png),
  [desktop point editor](stage1-points-desktop.png) and [compact point editor](stage1-points-mobile.png).
- [Final scoped performance report](stage1-performance.json) and
  [earlier full-run measurements](stage1-performance-before-optimization.json).

SHA-256:

| Artifact | Hash |
| --- | --- |
| Generated SVG | `62f86909df48bc32fee36861df262abed26fb6bc8697a8730126f1717c74cd76` |
| Generated DXF | `cfb12c0b8adf78b1e56285dd90e83434d33a4ec151f8c8cd8cecf9ba97985c9f` |
| Fixed SVG | `37801ef9fb4ab4598beda7633aa58e57047d53f1855ebd6d24ba452685df4fab` |
| Invalid fixed example | `6230820679fec07ea137cd1df910ce2ca5a791fa47aebc52523873fe57b1aaae` |

These are artifact/browser checks. Independent CAD reimport with real curved
segments belongs to Stage 4.

## Verification

| Check | Result |
| --- | --- |
| Full unit suite, `pnpm run test` | Passed core, renderer, Ergogen, KiCad, CAD and all 440 app tests before the final isolated core optimization. |
| Final core suite, `cargo test --manifest-path core/Cargo.toml --locked` | 308 passed, 6 existing ignored. Includes red-before/green-after recess and closure-clearance regressions, version lifecycle, rounded authored holes, missing-source recovery and scoped support/readiness. |
| Production build | Root build passed; final core WASM and app build passed after the optimization. CAD and release renderer artifacts were built and checked separately. |
| Contract generation/check and runtime import | Passed; generated TypeScript comes from Rust. |
| Native/WASM boundary parity | Passed 13 core and 9 archive requests on final core WASM. |
| Outline browser workflows | All 16 outline scenarios pass on the final build, including active SVG/DXF download, fixed-copy export, exact input, gestures/cancellation, Undo, protection after movement, reopening, findings and save/export gating. The export test was corrected to reopen the outline inspector after leaving Export. |
| Affected Case workflows after clearance-cutter correction | 4 passed: layer visibility/readiness, adopted gasket closures, GH60 and Discipline case generation/export readiness. |
| Security audit | Passed with the repository's existing allowed cgmath warning. |
| Diff whitespace | Passed. |
| Workbench and pointer/matrix/outline latency | Final scoped run uses the existing five-session/100-sample medians, host/browser and budgets, plus unchanged interaction tests. The separate live Case gate is not included in this pass. |
| Final in-app visual review | Passed on the final production preview: repaired controller/thumb edges, Show gap highlighting, shared Outline/matrix bridge selection and point editor opening/Done without creating a version. Generated is active and original component placement is restored for user testing. |

The full browser suite was run before the final closure-cutter correction:
217 passed and 13 failed. Four failures were caused by treating case cutters as
supported components; the correction and all four targeted workflows now pass.
The remaining broader gate failures are retained separately, not counted as an
outline pass or silently waived:

- `pnpm run check:repo`: pre-existing unused `CaseChoice` export. The starting
  `HEAD` contains the same export without an import consumer; its source is unchanged.
- Case preparation: asynchronous renderer cold-start measure was read before it
  existed (`NaN`). The renderer timing implementation is unchanged.
- Five Case edit workflows target Wall thickness while its existing Dimensions
  & clearances disclosure is closed. The inspector implementation is unchanged.
- Two unchanged gasket fixture workflows expect 12 supports but receive zero.
  Their cause is not fully diagnosed; these remain failed gates. The fixture,
  expectation and mechanical generator were left unchanged.
- Generated mechanical package test uses the previous export action selector.
- Pages subpath gate waits for the previous Generate action; current Case UI uses
  Update preview. It was run and failed at that selector.
- Full performance runner measures workbench/interaction checks, then blocks at
  the same undisclosed Wall thickness control in the separate live Case harness.
  The final scoped report contains only the workbench and interaction gates and
  does not claim aggregate performance acceptance.

`pnpm run check` is therefore not fully green. Unrelated CAD-performance PLAN/TODO,
mechanical code, baseline budgets and source harnesses remain intact.

Final scoped performance (median of five session p95s; 100 samples per session):

| Scenario | Worker p95 | Painted p95 | Result against existing +10% baseline |
| --- | --- | --- | --- |
| 100-key single | 4.4 ms | 33.9 ms | Passed |
| 100-key row | 4.0 ms | 33.7 ms | Passed |
| 200-key single | 6.6 ms | 33.7 ms | Passed |
| 200-key row | 5.3 ms | 34.1 ms | Passed |

Outline preview p95 was 33.9 ms for 100 keys and 33.8 ms for 200 keys, against
the unchanged 100/200 ms targets. Matrix and pointer interaction tests also
passed. The scoped run retained the existing runner's session/sample counts,
budget comparison and interaction tests, ending before its blocked live Case
harness. It is a partial gate result, not a replacement for `pnpm test:perf`.

## User feedback follow-up: 2026-09-30

The user reported missing edge/alignment helpers, independent layout/outline grid
settings, obstructive point graphics and an unexpected hole after adding material.
These issues are addressed; acceptance still requires the user's retest.

- Layout, drawing and perimeter editing share the same Snap menu and grid state.
  Existing unit defaults remain; Off, 1 mm, 0.5 mm and 0.1 mm are also available.
  Changing the inspector grid changes the layout grid too. Geometry snap and Alt
  bypass apply to every outline gesture; exact coordinate entry remains independent.
- Drawing previews, placed points and perimeter drags use one inference resolver.
  Horizontal/vertical alignment, perimeter edges, collinear continuation and
  perpendicular directions are visible guides. Acquisition is 7 screen pixels;
  held guides release at 1.8 times that distance. Alt immediately releases guides
  and the grid even without another pointer movement. Drag targets remain frozen
  during previews so the moving vertex cannot become its own snap target.
- Visible handles are 3 pixels (4 selected); hit targets remain larger. Numbers
  appear for selected/hovered points, and midpoint insertion marks appear on
  hover/focus. Guidance labels sit to the left of the cursor for right-edge work.
- The material union could close a recess while leaving its interior as a hole.
  Fixed-copy additions now fill newly enclosed material and restore existing
  cutouts even when an addition overlaps them. This is scoped to fixed versions;
  legacy/unowned scripting retains its existing union topology. Authored cutouts
  still use Draw cutout. Existing saved openings are not silently deleted.

[Drawing alignment](stage1-inference-drawing.png),
[perimeter alignment](stage1-inference-perimeter.png) and
[compact point graphics](stage1-compact-points.png) show the updated controls.
[Machine-readable results](stage1-feedback.json) record 308 core tests, 444 app
tests, 16 outline browser scenarios, 8 layout browser scenarios, boundary parity
and passing 100/200-key outline latency (33.4/33.6 ms p95). The initial snapping
browser test and enclosed-recess native test failed before their fixes. The
stronger overlapping-cutout test also failed before cutout restoration.

A cold native archive-driver rebuild exceeded the app test's existing 5-second
timeout. Prebuilding the unchanged test driver allowed all 444 app tests to pass;
the timeout and harness remain unchanged. Repository checking still reports only
the existing unused CaseChoice export. This follow-up does not claim the broader
Case/fixture gates or full performance runner are green.

The in-app review preserved the user's saved Plaid fixed version. Further edge
routing, explicit angle/dimension controls and configurable grid origins remain
in Stage 3; inference guides do not add persisted sketch relationships.

## Human walkthrough

Open REVIUNG41 at `http://127.0.0.1:4328/`, select Outline in Objects, then:

1. Compare both thumb junctions and the controller/reset connection with the
   original screenshot. The marked recesses should be filled; the centre valley
   and staggered edge alternatives should remain.
2. Use Show gap to locate a recess. Toggle Keep gap, reopen the project, move a
   source component slightly and remove protection again. Undo the experiment.
3. Select a bridge under Outline, then its reference under right keys. Both
   selections should highlight the same material.
4. Open Edit perimeter points and cancel/Done without editing: Generated should
   remain the only version. Change one coordinate: a fixed copy should activate.
   Rename, copy, switch, delete and Undo; Generated should remain available.
5. Move U1 outside the fixed copy. Focus its support finding, continue editing
   and save/reopen. Affected fabrication exports should block. Switch to Generated
   to restore valid output; Undo your placement experiment.

Record feedback and acceptance here after the user's walkthrough. Suggestions,
linked refinement/Freeze and ghost comparison are Stage 2; connected edge/path
tools and additional precision controls are Stage 3; curved CAD output is Stage 4.
