# Proof and execution contract

Parent: [map.md](map.md). Ticket proof rows are the acceptance boundary, in repository format:
`requirement — precondition (critical timing) → exact action → independent oracle · tier · status`.
All implementation rows are **planned**, not RED, GREEN or reused. A test file/graph edge is
candidate reach, not a passing receipt. Frozen prototype results are not production proof.

## Reuse first and keep stories distinct

At implementation, consult Gortex `change.tests` for the actual changed symbol IDs and read
relevant existing assertions. Planning queries for LayoutContextActionToolbar, KeymapPanel
and CasePanel returned no reaching targets; that does not prove they are untested. Existing
mounted modules are visible in their source. Do not use that graph gap as a reason to duplicate
fixtures. Candidates include:

| Slice | Existing owner/fixture candidates (not executed in planning) |
|---|---|
| Shared host / keyboard | `ui-shared/src/panels.rs`, Layout popup helpers and their mounted tests |
| Retained draft update safety | `runtime/src/draft_owners.rs` registry tests plus existing registered-editor/update-safety fixtures; page-host wiring needs its own mounted receipt |
| Layout tools | `layout/src/objects/{layout_toolbar,layout_transform_toolbar,layout_align}.rs`, existing snap/part-rail tests |
| Layout editor ownership | `web/src/presentation/{layout_draft_safety_tests,layout_component_inspector_tests,layout_field_preview_tests,layout_relation_highlight_tests,layout_split_focus_tests}.rs`, outline mounted tests |
| Setup / Add | existing matrix setup/preview, mirrored-pair, existing-half and component-placement tests |
| PCB | `pcb/src/pcb_wiring/{mode_owner_tests,queued_edit_tests}.rs`, existing module/selection/remap tests |
| Keymap | `keymap/src/keymap/panel.rs` disclosure tests plus binding/macro/encoder/layer-controller tests |
| Keycaps | `web/src/presentation/keycaps_workspace_tests.rs` and crate settings/fit tests |
| Case | `case/src/cad_presentation.rs` mounted tests; `presentation/case_workspace.rs` scope/tree/display tests |
| Parts | `parts/src/parts.rs` catalog, inspector, preview/placement and stale-owner tests |

Parameterize equivalent widths/appearances/selection inputs. Keep timing failures separate:
closed→reopen, owner A→B→A, stale/deleted owner, panel switch and workspace round trip do not
necessarily fail together. Common mechanics get one faithful shared-host test; repeat only
adapter wiring/admission where independently breakable. One small accepted-document fixture
per domain can exercise several inventory entries; do not generate a six-workspace × every
width × every field × every Theme end-to-end product. Native accepted-state tests retain
edit/undo invariants; mounted tests prove actual DOM/interaction; packaged checks prove real
routing/assets and visual/accessibility integration. Do not widen production visibility for tests.

## Commands (from current README/runner)

Run from the planning/implementation worktree, never the dirty original checkout. Redirect
long output to gitignored `.evidence/`, logs only. Do not override build flags or launch two
build owners for a target. The coordinator alone runs full build/suite gates.

```sh
# Planning/doc-only checks
python3 scripts/check.py repo
python3 scripts/check-doc-links.py

# Per implementation slice: affected checks, including wasm-gated lint
python3 scripts/check.py quick
python3 scripts/check.py lint

# Focused mounted crate launches; each ticket selects its named crate(s)
python3 scripts/check.py browser --only ui-shared layout
python3 scripts/check.py browser --only pcb
python3 scripts/check.py browser --only keymap
python3 scripts/check.py browser --only keycaps
python3 scripts/check.py browser --only case
python3 scripts/check.py browser --only parts

# Focused page suites; workbench_shell.rs is the proposed new private host (009)
python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_draft_safety_tests.rs web/src/presentation/layout_component_inspector_tests.rs
python3 scripts/run-wasm-tests.py --files web/src/presentation/keycaps_workspace_tests.rs

# Acceptance 007: fresh packaged page + mounted/browser smoke
python3 scripts/check.py browser

# One stable M1 Layout candidate before hands-on review; repeat once at final008 retirement
# Coordinator only, never concurrent with a delegate's Cargo/wasm-pack target owner.
python3 scripts/check.py
```

`browser --only` is explicitly a partial subset: no full packaged build or drag smoke.
`run-wasm-tests.py` requires `--files ...` or `--all`, not the old no-argument placeholders.
Capture a receipt when handing off implementation evidence; label newly executed vs reused
and source identity. Apply native test-name guard updates only with the causing rename,
reviewed as a name diff. Never waive required counts/assertions to make retirement pass.

## Mutation boundary

For a row that is RED, record the actual failure and expected assertion before fixing.
For an uncovered requirement that already passes the baseline, record GREEN + `proof gap`
and run its named mutation below. Existing sufficient evidence may be reused with a receipt;
do not manufacture RED. Mutations run through `scripts/mutation-check.py`, not ad-hoc edits.
Create exact unique-text mutation specs against the implemented source (not speculative
source copies); keep specs with the implementation ticket, logs only in `.evidence/`.

Only rows the builder recorded as `proof gap` get a mutation; a `RED` row already showed its test can fail.
Command form, replacing SPEC with that ticket's actual spec path:
`python3 scripts/mutation-check.py SPEC --locked --command "<row runner below>" --evidence-dir .evidence/mutations`
(`--locked` takes the build lock per run; do not wrap the script in an outer lock).
The runner's exit zero means restoration succeeded, not that every mutation was killed.
Read each result: killed at the named assertion counts; compile failure, timeout or a
failure elsewhere does not. Restore unchanged bytes. An unmutated run must be green.

| Ticket / row family | Named semantic mutation if proof gap | Runner after `--command` / tier | Required failing oracle |
|---|---|---|---|
| 009 exclusion/focus | permit independent Objects+Inspect visibility; remove inert/return-focus one at a time | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` / mounted | exactly one visible panel; hidden descendants not tabbable; opener focused |
| 009 transient interface | ignore a typed owner's hide refusal or mirror its closed state in the host (separate mutants on labelled content through the production interface) | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` / mounted | refused hide changes neither owner; own close never reopens an old panel; no two visible surfaces |
| 009 navigation admission | force a refused domain route or retain old focus after accepted route (separate mutants) | same / mounted | refused route/state unchanged; accepted route has no old callback/focus authority |
| 009 generic fitting | measure the pre-change fixture or leave its probe interactive | same / mounted | final measured allocation before movement; probe never focusable |
| 009 reframe | subtract pop-out width from canvas | same / mounted | canvas rect/camera unchanged |
| 002 motion/measurement | fade a label; measure pre-toggle; ignore reduced motion (separate mutations) | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` / mounted | constant opacity/unit offset; final fit before first moving frame; zero animations |
| 002 guards | enable first-column Spread or Follow-off Rotate (separate mutations) | `python3 scripts/check.py browser --only layout` / mounted | unavailable action cannot activate and accepted state is unchanged |
| 003 draft lifetime | reset owner A's draft on close/owner switch | `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_draft_safety_tests.rs` / mounted | A's typed text/error restored; no B commit |
| 003 hidden registered Inspector | drop the actual Inspector owner's guard on close while retaining its text | `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_draft_safety_tests.rs` / mounted | Update Now is deferred, names Layout/editor/remedy and does not reload over the hidden unresolved draft |
| 004 active outline / half | activate a version on pick; change link source on half-focus (separate mutations) | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs web/src/presentation/layout_split_focus_tests.rs` / mounted | active version/shape and source/link/Undo unchanged |
| 004 reference | swap row/column or use screen-side name for anchor reference | same / mounted | literal known anchor reference matches canvas readout |
| 005 reach/error | remove a new Add entry; drop failed-placement message (separate mutations) | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs web/src/presentation/header.rs` / mounted | retained command is reachable; failure stays visible without an accepted edit |
| 005 hidden registered draft | retire real MatrixSetup/script guard on hide but retain text | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` / mounted | Update Now names Layout/editor/remedy and cannot reload until that owner's existing submit/Cancel resolves it |
| 005 registration scope | restore old-scope predicate/registration after replacement | same / mounted | old owner does not block new session; its current unresolved owner still does |
| 005 pending task admission | clear real Create admission on hide/show | same / mounted | no second Create submission or refreshed admission; existing captured first result settles once |
| 005 domain close | leave task visible after its owner Cancel or force a new host panel after a refusal | same / mounted | domain/host visibility agrees, no old panel reopen or synthetic cancellation |
| 006 mixed-shell route | dispatch Export/shortcut reveal to the old active workspace rather than its captured destination | `python3 scripts/run-wasm-tests.py --files web/src/presentation/export_workspace.rs web/src/presentation/workspace_navigation.rs web/src/presentation/workbench_shell.rs` / mounted | correct current target authority only, original selection clearing/focus guard preserved |
| 006 header/save | suppress failed-save presentation or report Saved after failure | `python3 scripts/run-wasm-tests.py --files web/src/presentation/header.rs` / mounted | failed status + recovery actions visible, never Saved |
| 010 PCB pane | unmount Routing & nets when Selection is shown | `python3 scripts/run-wasm-tests.py --files web/src/presentation/pcb_workspace.rs` / mounted | return to prior pane preserves its draft/feedback |
| 011 Keymap admission | omit captured input identity/generation check | `python3 scripts/check.py browser --only keymap` / mounted | late response cannot bind replacement key/encoder |
| 011 dirty layer name | bind delayed rename to current B, or drop A's retained dirty flag (separate mutants) | same / mounted | B unchanged AND A is exactly renamed once after valid blur or retains its full draft/error after refused/no trigger; lost draft cannot pass |
| 012 fit ownership | accept old finding navigation after board/instance change | `python3 scripts/run-wasm-tests.py --files web/src/presentation/keycaps_workspace_tests.rs` / mounted | no foreign selection/camera change |
| 013 Case View | route Section to Exploded | `python3 scripts/check.py browser --only case` / mounted | existing Section transient/rendered result, no accepted edit |
| 015 Case owner | mount a second automatic observer/command registration | same / mounted | one real generation request and one registration per command |
| 015 Case currency | allow previous/foreign result to authorize Export geometry | same / mounted | export remains disabled/no delivery |
| 014 Parts admission | ignore selection generation for placement/import completion | `python3 scripts/check.py browser --only parts` / mounted | no placement/draft write into different selection |
| 007 contrast | substitute failing foreground in one Theme/Appearance | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` / mounted | rendered production-role 4.5:1 text / 3:1 large text and relevant UI pairs fail |
| 008 upgrade | read obsolete dock width/mode into new host | `python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` / mounted | no phantom dock/width, unrelated Theme retained |

Page-host/adapter mutants require the named page suite: a crate-only launch cannot prove
wiring in `web/src`. Place new host interaction assertions in the proposed private module;
reuse existing domain fixtures/assertions rather than duplicate whole workflows there.
For draft safety, reuse native registry pruning coverage; the new page proof checks the
actual retained editor/guard wiring, not a test-only registry or a copy of its predicates.
[Refactoring preparation](refactoring.md) fixes M1's exact native-blur versus no-trigger
preconditions. M2 first classifies actual unregistered forms; do not presume an explicit-
submit draft or weaken required draft-retention outcomes.
For every `same` entry above, use the immediately preceding explicit runner.

For other newly identified proof gaps, name the precise mutation/runner/assertion in the
same ticket before building; changing the approved boundary needs the user. Each adapter's
stale-owner reject path is rerun independently by the coordinator after delegated work.

## Closure and limits

Per ticket: RED/proof-gap evidence → fix → affected checks → named boundary mutations →
one fresh combined Spec+Standards reviewer → integrated ticket gate. Report unread files,
mandatory unmet criteria and follow-ups separately. No approval is waived by a reviewer.
007's packaged visual/keyboard checks use agent-browser and both Themes/Appearances; fresh
text, icon, focus and SVG-chrome contrast measurements replace incomplete prototype scans.
Mandatory accessibility failures block; unrelated findings need a separately triaged record,
not a blanket exception. Browser checks still do not certify fabrication or CAD validity.
