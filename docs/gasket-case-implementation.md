# Internal gasket implementation

This checkout implements the Rust/CAD construction and direct editor wiring from
[gasket-case-redesign.md](gasket-case-redesign.md). Selecting Gasket mount enables
the internal construction. No conversion workflow is included, following the
user's explicit scope correction. Verified hardware catalog selection remains
separate work.

## Editor controls and validation

The Case editor exposes all 13 foam sizes and custom dimensions, compression,
travel, support count, advanced clearances, custom closure hardware, and the
resolved purchase/cut list. Edits use existing undo/redo and save behavior.
Adopted closures remain independent of support-count changes. The generated top
is labeled Top case. Invalid geometry blocks export with diagnostics.

Validation of this editor phase: all 401 app unit tests, the full core Rust suite,
application type checking, and the rebuilt production app passed. Three browser
regressions passed against the rebuilt WASM: generation and save/reopen with
undo/redo; Escape cancellation and custom edits; closure preservation and invalid
wall export blocking. This last test exposed and fixed a generic mount check
that incorrectly tested internal case screws against the floating plate.

The default hardware is explicitly Custom M2 geometry, not a supplier-qualified
preset. The earlier repository-wide browser failures below remain outstanding.

## Mixed placement and assembly editing

Automatic placement now tries 80, 70, 60, 50, 40, 30, 20, 10 and finally 5 mm
cuts independently for each support, with at least one support facing each of
four sides per region. Saved supports carry independent length and width and
remain pinned. Support labels retain stable slot order after a manual edit.
The original CAD fixtures explicitly select fixed sizing for regression coverage.

Editing tracks cover the complete perimeter. A drag can cross the board directly;
fit is checked on release. Invalid supports retain their position and dimensions,
show a red handle and a fit message, and block generation/export. The previous
solid remains visibly stale until the configuration becomes valid.

The Case tree now shows the assembly, a collapsed PCB component branch, solid
parts and a gasket group. Selecting a solid or gasket focuses its inspector.
Individual pads expose cut length and width; common foam thickness, compression
and clearances live in the gasket group. Physical assembly setup stays in the
assembly inspector. Upper/lower pads remain a matched support pair.

Validation: 403 app tests and 273 core Rust tests passed. Four browser regressions
cover focused inspectors, individual invalid-size persistence and repair,
undo/redo, shared material edits, adopted closure preservation and wall checks.
Six CAD export fixtures, including mixed
cut lengths, passed connected-solid and STEP reimport checks. Live Sofle testing
confirmed mixed 10/30/50 mm cuts on four sides and direct cross-board placement
with retained red handles and blocked export. This does not establish physical fit.

## Split assemblies and display controls

Case Objects lists both physical assemblies. Selecting an assembly switches the
canvas and generation context to that half, including reversible designs that
share a PCB. PCB reference components appear last and collapsed; gaskets are a
collapsed group with individual controls. Visibility and colour are local view
preferences per project/physical instance and do not invalidate CAD.

Mechanical construction and closure hardware are shared across both assemblies.
Gasket positions, openings, battery placement and adopted closures remain local.
Top and bottom inspectors link to the shared closure editor. New custom hardware
starts with a 3 mm M2 insert; nominal M2/M2.5/M3 size choices match the supplied
examples. Seat and screw dimensions remain editable starting dimensions, not
supplier-qualified installation specifications.

Automatic placement reserves space for later supports and balances the support
count across four sides before selecting descending cut lengths. A dense
12-support rectangle regression reproduces the previous overlapping layout and
now passes with three supports on every side. Existing manually positioned pads
are retained; Reset gasket placement explicitly releases them.

The renderer supports adjustable exploded separation, XY/XZ/YZ section planes,
a position slider and a translucent plane indicator. Hidden-line mode draws
occluded edges without revealing objects that were hidden. Picking uses the same
section equation and exploded displacement as rendering.

A split-assembly browser regression exposed a queued-edit revision bug: the
internal document replacement for mechanical edits used an outdated revision.
Such replacements now use the current revision at execution time; explicit full
document replacements retain their original revision guard.

Validation for this refinement: 406 app tests passed with two workers, 274 core
Rust tests passed (including the dense-layout regression), and 26 renderer tests
passed. Six focused browser tests cover saved edits, visibility/colour and view
controls, and generation on both Sofle halves while sharing insert changes.
The colour test also verifies a rendered-pixel change without a CAD revision
change. Six internal-gasket CAD export/STEP-reimport fixtures and native/WASM
boundary parity passed. Desktop and narrow live previews were inspected.
The existing unrelated repository-wide browser failures below were not rerun.

## Implemented contract

Rust owns the independent PCB-based exterior, nominal plate and local tabs,
matched upper/lower foam stacks, support tracks, closure positions, screw
selection, blind seats, optional downward bosses, and material/hardware lists.
The top retains the persisted `retainer` body ID.

Custom hardware supplies explicit dimensions, screw-length datum, installation
method, and available or fixed lengths. Unresolved datums and incompatible
processes block generation. Adhesive thickness is not compressed. Unknown foam
compression data produces a warning rather than an export approval gate.

Generated supports can be repaired or removed by a count change. Saved
user-positioned supports and adopted closure locations remain protected.
Split-region closures and linked support placement are handled independently.

Prepared CAD carries typed support prisms, cylindrical seats, and conical seats.
Native construction checks complete support footprint ownership, fuses additions,
cuts seats and openings, and rejects disconnected feature results. Feature
geometry participates in cache identity and bypasses the planar shortcut.

## Validation scope

Public Rust tests cover geometry, persistence, travel, process compatibility,
screw selection, adhesive behavior, and all 13 specified foam sizes. Native CAD
fixtures cover rectangular, countersunk, downward-boss, rotated-concave, and split
cases. They check connected solids, STEP reimport volume, tower material, and
plate/PCB clearance at both travel limits. These fixtures use synthetic Custom
hardware dimensions, not verified product presets.

Validation completed so far:

- Core Rust suites: 271 tests passed, including 21 internal-gasket regressions.
- Renderer Rust suite: 25 tests passed.
- Native CAD: 20 tests passed; four diagnostic benchmarks remain intentionally
  ignored by the normal suite.
- CAD JavaScript/export suite: 44 tests passed, including all five new fixtures.
- Application unit suite: 399 tests passed across 67 files.
- Repository structure, generated contracts, and runtime import checks passed.

The production build, CAD type check, and native/WASM boundary parity checks
passed. The full `pnpm run check` returned failure at the browser suite:
**193 passed, 8 failed**. The failures were:

| Test | Observed failure |
| --- | --- |
| `cad-loading.spec.ts:4` | Clicks old “Generate” label |
| `electrical-handoff.spec.ts:8` | Expects old “Generate” label |
| `workbench.spec.ts:81` | Clicks old “Generate” label |
| `workbench.spec.ts:465` | Clicks old “Generate” label |
| `case-preparation.spec.ts:4` | Expects profiling measures without `?cadMetrics=1` |
| `parts-catalog.spec.ts:6` | Expects 37 entries; catalog contains 39 |
| `parts-catalog.spec.ts:45` | Expects 37 entries; catalog contains 39 |
| `workbench.spec.ts:403` | Expects hidden “Saved locally” text to be visible |

The corresponding UI label (“Update preview”), profiling guard, catalog, and
save-status UI are unchanged from baseline commit
`49f88a36776a95a5288edb2366803fc61771ab05`. This is a source comparison, not a
separate baseline browser run. No frontend test or production UI changes were
made in this phase. The separate `pnpm run test:e2e:pages` was run after the
main command stopped and also failed: `e2e-pages/deployment.spec.ts:3` waits
for the old “Design” tab. There are nine browser failures across both runs.

These checks do not establish installed-app behavior, physical fit, or
pointer-to-paint timing.

## Remaining work

- Complete verified catalog families and automatic family ranking/defaults.
- Run the specification's live interaction and performance acceptance sessions.
- Validate actual selected hardware and foam against their product data before
  claiming those catalog entries complete.

The original checkout's performance experiments remain untouched. Its historical
Phase 3 execution record is named in the design but is not copied into this
isolated implementation.

## Independent review

Standards review found no remaining hard violations or concrete correctness
issues after fixing support ownership and full CAD footprint validation.

Spec review found no additional reproducible defect in this Rust/CAD slice.
Catalog selection and full live acceptance remain incomplete as
listed above. An initially suspected saved-closure cap collision could not be
reproduced independently of existing rejection checks, so no speculative change
was added.

The branch is an implementation checkpoint, not a claim that the full redesign
or the full repository acceptance gate is complete.

## Automatic support count follow-up

Gasket count now follows outline perimeter, targeting one support pair per
50 mm and rounding up to balanced groups of four (4–64 pairs per region).
The longest region determines the shared slot count, keeping linked regions
compatible. Sofle resolves to 12 pairs instead of the previous four.
This is a placement heuristic; it does not establish physical load capacity.

The Gaskets inspector exposes automatic count and a manual Supports per region
field. Changing the count selects manual mode; disabling automatic count keeps
the currently resolved count. Previously saved default counts of four use the
new automatic behavior, while older non-default counts remain manual. Saved
user-positioned support slots are retained when automatic count is enabled.

Validation: 277 core tests and 406 app tests passed, along with generated
contract checks, app/CAD typechecking, and native/WASM boundary parity.
All seven gasket browser tests passed, including Sofle automatic count, manual
count persistence, and switching back to automatic mode. Seven native CAD export
fixtures passed, including connected tray/top solids with automatic count.
Independent standards and specification reviews found no confirmed issues.
The broader repository acceptance limitations recorded above still apply.

## General straight-run placement follow-up

The perimeter-count heuristic above is superseded by a geometry-driven planner.
Automatic count is now the result of fitting supports to usable straight runs,
not a perimeter-derived quota shared equally among four compass sides. Tiny
jogs within 0.1 mm can belong to one run; every resulting pad still passes the
existing full geometry and collision checks against the actual outline.

The planner centers long cuts, subdivides longer runs into equal-length groups,
and visits separate ledges and angled runs. Exact centering is preferred for
pairs; single-pad placement balances cut length against distance from the run
center. Cut sizes remain 10 mm increments up to 80 mm, with 5 mm as a fallback.
Explicit counts prioritize four-side coverage, then subdivide the least-supported
runs. Pinned supports own their runs and retain their saved IDs and anchors.

Suggested closures prefer ends of longer runs, with a cost for occupying the
longest uninterrupted span. This leaves short ledges available for supports.
Adopted closure positions remain fixed. The planner has no board names or
board-specific coordinates; Sofle coordinates appear only in regression tests.

New regression coverage includes a long centered run across a tiny outline jog,
symmetric separated pairs, the missing Sofle ledges, and rectangle/concave/split
outlines after rotation, translation, and winding reversal. This remains a
placement heuristic, not a claim of verified physical load capacity.

Validation for this follow-up: 280 core tests, 406 app tests, seven gasket browser
tests, and eight native CAD export cases passed. App/CAD typechecking, contract
checks, native/WASM boundary parity, and the production preview build passed.
Both independent review axes reported no confirmed issues. The full-repository
acceptance limitations recorded earlier remain outside this follow-up.
