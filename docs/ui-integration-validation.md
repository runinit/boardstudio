# UI integration validation

Integration branch: `codex/integrate-ui-simplification`, based on `dev` at
`29b5adb4`. UI source: `/home/chris/.codex/worktrees/f357/ts-boardstudio2`,
based on `0ca5e3f1`, with 47 modified and 11 untracked files captured before edits.
Snapshot, hashes, and execution logs: `/tmp/boardstudio-ui-integration-evidence`.
The source worktree is not modified. The unrelated CAD performance plan is excluded.

## Preservation map

| Capability | Evidence to retain and run |
| --- | --- |
| Guided setup, controller placement, reversible layout, cancellation/resume | `setup-workflow.spec.ts`, `setup-guide.spec.ts`, `workflow-overhaul.spec.ts`, `projectConstruction.test.ts` |
| Recovery, reset, saved projects, demo copies | `startup-recovery.spec.ts`, `reset-projects.spec.ts`, `project-library.spec.ts`, `storageReset.test.ts`, `createProjectActions.test.ts` |
| Matrix placement, resize, projections, outline editing | `matrix-*.spec.ts`, `canvas-interactions.spec.ts`, `outline.spec.ts`, `placementGeometry.test.ts` |
| PCB wiring, footprints, assignments, export artifacts | `electrical-handoff.spec.ts`, `kicad-artifact.spec.ts`, `electricalWorkflow.test.ts`, Rust electrical tests, KiCad package tests |
| Case defaults, mounting, battery/stabilizer clearance, physical instances | `setup-workflow.spec.ts`, `mechanical-assembly.spec.ts`, `case-*.spec.ts`, `mechanicalDefaults.test.ts`, `closureClearance.test.ts`, Rust mechanical tests |
| Simplified header, navigation, contextual objects, returns | `header-consolidation.spec.ts`, `workflow-overhaul.spec.ts`, `workbench*.spec.ts`, rendered Impeccable review |

## Baseline findings

- Unmodified `dev`: app suite initially 335 passed, one storage archive test
  timed out under concurrent test load. Focused rerun passed all 11 storage tests.
- Unmodified `dev`: `pnpm check` reached Ergogen runtime tests and failed on
  expected catalogue count 37 versus actual 36. `dev` intentionally retired
  `infused-kim/nice_nano_pretty`; update the assertion to 36 and explicitly assert
  the retired controller remains absent. No production catalogue change needed.
- Unmodified `dev`: the Parts browser count expected 37 but rendered 38;
  mirrored component persistence also assumed canonical IDs instead of dev's
  construction snapshots. Both failures were reproduced on unchanged dev.
  Assertions now check 38 products and the saved LED generator/reversibility,
  while retaining identity and geometry assertions across reload.
- Initial integration typecheck lacked generated WASM packages in the new
  checkout; the normal build supplies them before validation.

## Reconciliation decisions

- Keep `dev`'s stage-aware guided placement, which handles both matrix and
  controller return paths, instead of adding a second independent guide flag.
- Retain `dev`'s reset/recovery, case instance controls, reversible construction,
  and mechanical operations. Add saved-project opening alongside them.
- Keep the dedicated export page and contextual Back action. Preserve Escape
  and export-toggle closing behavior through the existing close handler.
- Place destructive local reset in Workspace settings, retaining confirmation
  and atomic storage behavior without cluttering the keyboard browser.
- Adapt tests to the new navigation and existing `dev` control labels; retain
  functional assertions and visual thresholds.

## Integration regressions fixed

- Damaged stored project metadata crashed the new gallery. A failing component
  test reproduced it; the gallery now preserves access to other projects and
  offers a preview-unavailable tile. Storage failure/retry is also covered.
- A failed save while opening a stored keyboard left the core on the new document
  while the UI retained the old one. A failing action test reproduced it; the
  previous core document is restored when acceptance fails.
- Queued mechanical commands constructed replacements from the current document
  but used an older caller revision. The mechanical browser test exposed stale
  revision errors; a deterministic unit regression failed before the correction.
  Caller-authored full-document replacements still retain stale-revision checks.
- The guide and empty canvas duplicated Add key matrix. Existing onboarding tests
  caught it; the guide now owns creation while visible.
- Generated controls appeared alongside authored controls during initial
  configuration saves. A failing component test now protects the single action
  region until the configuration is committed.
- Automatic mounting defaults could be queued while Generate was enabled.
  A failing component test now protects the save transition: generation stays
  disabled with an explicit saving message until those defaults commit.
- One remaining Guide selector now follows Project → Setup guide. Navigation
  and label adaptations preserve all original functional assertions.
- A second retired-controller count assertion in KiCad was also stale on dev;
  reproduced directly on dev and corrected from 37 to 36 in integration.

## Impeccable audit, hardening, and polish

Implementation integrity: pass for the scoped integration. The simplified header,
contextual objects, settings, keyboard browser, and dedicated export page remain.

| Dimension | Score | Evidence and limits |
| --- | --- | --- |
| Accessibility | 3/4 | Keyboard export/return and focus restoration exercised; labels and focus paths covered by browser tests. No full assistive-technology certification. |
| Performance | 3/4 | Demo previews use measured layouts without constructing full projects; generation engines unchanged. Existing large-chunk build warning remains. |
| Responsive | 3/4 | Desktop and 390px rendered inspection; automated 320–1680px header and compact browser checks. Compact input sizes corrected to 16px. |
| Theming | 3/4 | Light/dark gallery, settings, and export inspected; existing semantic palette retained. |
| Implementation integrity | 4/4 | Simplification preserved; failure paths hardened; scoped diff and detector reviewed. |
| Total | 16/20 | Good; no observed blocking UI defects after scoped fixes. |

One detector run returned 13 advisories, no blocking findings. Secondary gallery
text now uses the 12px label token and the new controls use the established 4px
radius. Intentional browsing/navigation typography, compact input sizing, and the
transient overlay shadow are documented in `docs/design/workflow-overhaul.md`.
No global detector configuration was changed. Two bounded rendered review rounds
covered desktop/mobile and light/dark surfaces. Development hot reload emitted
React createRoot warnings after module updates; those are not production-build
findings. Functional regression checks remain separate from this visual review.
Final DOM verification confirmed both mobile name/search inputs at 16px, no
horizontal overflow, and canvas zoom changing from 100% to 120%. Browser page
zoom and a full assistive-technology audit were not separately certified.

## Final verification

- Dev baseline: 20 focused browser tests passed.
- Integration: 35 focused browser tests passed after reconciliation.
- Final integration app suite: 364 tests passed across 67 files.
- Final targeted production run: all four checks passed for board-switch drag
  cancellation, CAD cancel/retry/cache, assembly persistence, and STEP export
  geometry round trip.
- `pnpm check` passed repository/contract checks, Rust and package tests, the
  production build, CAD typechecking, Rust-boundary checks, and all 205 functional
  browser tests (4.7 minutes). Its final Pages stage failed only because its
  navigation selectors still named the old Design tab and Case tree item.
- Corrected those Pages selectors, preserving every functional assertion;
  `pnpm test:e2e:pages` then passed its single worker/CAD/model/offline-export
  smoke test. Repository checks, app typechecking, and `git diff --check` passed
  afterward. All gate stages have passing evidence; the entire expensive gate
  was not repeated after this test-only selector correction.
- Final hash verification: all 58 captured UI files unchanged in the source
  worktree; dev was still at `29b5adb4` during validation, with its unrelated plan intact.
- Remaining limitations: six existing ignored Rust timing/doc cases remain
  ignored. The separate `test:perf` benchmark, development-server suite, external
  KiCad CLI oracle, full browser page-zoom matrix, and assistive-technology audit
  were not run for this integration. Browser coverage is Chromium. Existing
  Rust/vendor warnings and the large-bundle build advisory remain unchanged.
- No unresolved integration regression was observed in the final checks.

## Integration provenance

Validation was performed before commit in
`/home/chris/.codex/worktrees/boardstudio-ui-integration` on
`codex/integrate-ui-simplification`, based on dev `29b5adb4`. The user subsequently
authorized committing and merging this integration into dev. Git history records
the resulting commit and branch ancestry. Preserve the source UI worktree and
the unrelated CAD performance plan when applying the integration.

Core, CAD, renderer, contracts, electrical planning, project session management,
and case-generation hooks retain dev implementation. Changes outside the imported
UI are limited to project browsing/storage, queued mechanical revision handling,
UI transition guards, regression tests, and documentation.

The browser artifact checks inspect actual KiCad board packages, wiring reports,
footprint metadata and transforms. The STEP export check reopens the downloaded
file through the CAD reader and checks nonempty finite geometry, normals, XY
extents, and the starter plate's 3mm thickness. CAD package tests independently
check solid validity, dimensions, volume, and multi-body round trips.

Intermediate browser attempts overlapped generated-package rebuilds, so their
transient reload/detachment failures are retained in the evidence logs rather
than treated as conclusive production results. Final production validation must
run after builds finish, without concurrent changes to served assets. No visual
thresholds or test timeouts were loosened. The incoming PCB layer pixel test
uses a 1280×800 viewport to retain drawing height under the workflow bar.
The board-switch drag test now waits for the new-board revision to commit before
capturing its baseline; its assertion that dragging does not save remains.

## Durable milestone

Project action `act_muk633u3_63e87b1f5245` saved the verified decisions and limits
in AgentMemory. Crystal generation is pending because the service rejected it
with a monthly quota error. No crystal or generated lessons were claimed saved.
