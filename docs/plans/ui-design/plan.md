# UI design lane

Parent: [140](../typed-core-edits/issues/140-ui-design-impeccable.md)
Status: active discovery; independent implementation branch, later merge

## Goal and authority

Make BoardStudio a sleeker, compact and clear expert workbench. Preserve precise direct editing and existing product truth while improving iconography with Solar, descriptions, hierarchy, color, responsiveness, contextual interactions and useful motion. Improve Layout editing and Case generation controls/visuals through existing shared operations. This is Operate-mode UI work, not a new keyboard implementation or a Core rewrite.

Coordinator: `01a12c41-1a51-7101-a555-188f1c85edce`; worktree `/home/chris/.codex/worktrees/2182/ts-boardstudio2`; branch `codex/ui-impeccable`; base integration `53970a404`. The coordinator asks the user questions directly in its own chat. It owns discovery, scoped child plans, Luna implementation and one fresh independent Sol medium finish/behavior review per coherent batch. Root owns shared tracking, index refresh, heavy scheduling and later integration. No recurring polling automation or routine permission roundtrips.

## Delivery sequence

1. **Observe and shape.** Run Impeccable context once per session; use critique/audit against real representative screens and task states. Reuse existing source critique packets where useful, but inspect rendered truth. Ask only consequential unresolved design questions in the lane chat. Establish a compact brief and agreed visual direction within current product constraints.
2. **Foundations and iconography.** Establish consistent spacing/density, type hierarchy, semantic color roles, focus/selection/disabled states and compact controls. Select a coherent Solar style and reusable accessible icon wrapper. Verify upstream provenance/license, local asset strategy and attribution; avoid importing unused sets or unrelated dependencies.
3. **Shell and Inspector.** Clarify navigation, project identity/save feedback, sections, descriptions and selection context. Reduce wasted space while keeping controls understandable and reachable; preserve accepted Inspector wording and Export blocker behavior. Handle narrow layouts without silently losing actions.
4. **Contextual Layout interactions.** Make current selection, available transforms and relevant actions clear and close to the task. Improve discoverability/feedback for existing matrix, stagger, offset, rotation, deletion and assembly controls. Preserve exact operation semantics, undo and measurement accuracy; do not invent future mirror APIs.
5. **Case workflow and visuals.** Improve generation parameter hierarchy, dimension/relationship explanations, preview readability and interaction feedback. Clearly distinguish active/pending/stale/failed geometry. Use existing case operations; CAD kernel or geometry defects require a separately reproduced scoped issue.
6. **Responsive, accessible and motion finish.** Check desktop and narrow viewports, keyboard/focus, overflow, touch targets, contrast, loading/errors and empty states. Use purposeful transitions with reduced-motion support. Avoid animations that delay editing or obscure geometry.

Each batch has exact paths, small acceptance criteria and an evidence packet. Use relevant Impeccable playbooks on demand (clarify, distill/layout/typeset, colorize, adapt/harden, animate, polish); do not run every command as a ritual. Read craft-floor immediately before UI edits. Follow the skill's bounded batched visual passes and stop open-ended polishing. Combine its fresh finish review with repository behavior acceptance when the same reviewer receives the complete evidence and rubric.

## Ownership and independence

Standing UI lease: `web/assets/` styling and narrowly selected Solar assets; presentation modules under `web/src/presentation/`; visual components/controls in the `web/crates/` UI crates; UI-local tests and lane-local design briefs. The coordinator records exact changed paths per batch before its writer starts. Shared business logic, runtime/transport, Core/model/solver/schema, CAD kernel, footprint generators, root goldens, scripts/check.py, maps/ADRs/CONTEXT are excluded. Lane-local DESIGN.md/Impeccable design artifacts may evolve after direct user design approval; main checkout's dirty documentation must remain untouched.

F138 is deleting demos and porting shared tests. Do not edit F-owned retirement test hunks, bring back demo dependencies or borrow their geometry as an oracle. Use independently authored synthetic user projects. E21/41 owns future mirrored web adapters/companion controls: reserve its actual API wiring for that lane; coordinate shared visual component overlap through root. Preserve accepted Export137 synthetic regression. Read the central lane registry before starting a batch and escalate actual ownership conflicts, not routine private UI decisions.

Read-only commands and disjoint source work can proceed concurrently. One writer per worktree. Builds/tests/browser/timing/index refresh require the single explicit GLOBAL HEAVY grant; request whole bounded check cycles. Use this worktree's real target/cache/pkg directories; do not share mutable build outputs with other lanes. Root alone bootstraps/refreshes its unique GitNexus index. Before bootstrap, use the explicitly named integration index as provisional graph evidence with source confirmation, never a false clean gate.

## Checks and later merge

Behavior fixes need meaningful expected RED then GREEN. UI presentation changes need actual WASM typecheck/browser validation, affected repo/lint checks and valid screenshots at the inspected viewports. Reuse sufficient source-bound evidence rather than repeat full suites per tweak. Capture exact revisions, changed bytes, paths, test results and open findings. Final complete graph delta and one independent review bind each coherent completed batch.

Keep reviewed commits on `codex/ui-impeccable`; do not merge or push. Root will choose a later integration boundary, sync with accepted Core/demo retirement changes, resolve real overlaps, preserve both regression sets and run one full integration batch. Report meaningful completion, failures and decisions; avoid constant progress messages. Parent140 remains open until its agreed scope is accepted and integrated.

## Discovery preparation, 2026-10-11

### Confirmed task priority

The user's priority is **faster Layout editing and contextual tools**. Current pain: common toolbar functions take too many clicks, snapping is hard to discover, and selecting an item, row or column should reveal more relevant actions. The first interaction batch therefore targets the Layout command surface and selection context. Icon/control foundations support that batch; they are not a separate broad cosmetic migration. Case follows after the Layout task improvements. The rendered baseline will identify the actual controls and existing operations to expose; preserve precise values, undo/redo, selection ownership and current snap semantics.

User steering now authorizes a persistent independent lane development build and migration of redesigned UI to Dioxus-supported Tailwind. The central workflow records the private-loop exception: private provider preparation, frontend rebuilds and visual inspection may run independently; full affected/acceptance suites, CAD validation and GitNexus refresh remain scheduled. All outputs stay in this worktree, with `CARGO_TARGET_DIR` set explicitly to its real `target/` directory. Proposed dev ports are 8182 (isolation proxy) and 8183 (Dioxus), subject to an availability check. Keep the server running after readiness and record its process handles and stop command.

Initial technical setup unit: `.gitignore` (exact generated CSS output only), `web/tailwind.css`, `web/src/presentation.rs` (stylesheet link only), and this lane's documentation. Dioxus CLI 0.7.10 upstream source selects Tailwind v4.1.5 when a Tailwind input exists without a v3 config. Use explicit `web/src` and `web/crates` Rust sources, existing theme variables, and no global Preflight reset. This authorizes pipeline setup, not an unreviewed visual redesign. Utility migration follows the rendered baseline and task direction.

Setup verified at `53970a404d2ddfcd07be55c5de682f1a832dcd22`. Impeccable context ran once in this checkout. Direct user task questions are answered above. Private initial provider preparation passed exit 0, then the detached dev server's first page build passed (168.69 s reported by Dioxus). The live preview is `http://127.0.0.1:8182/`, using Dioxus port 8183. Its live PID/process group and stop command are recorded in ignored `.scratch/ui-dev/runtime.json`; output logs are beside that receipt. Leave this server running as requested.

Private development validation: generated Tailwind v4.1.5 CSS exists and is linked once in the mounted app. CSS, Core worker entry and CAD WASM assets respond 200 with expected MIME and COOP/COEP headers; responses are recorded in `.scratch/ui-dev/asset-response.json`. A fresh project was created through **Create new keyboard**, and its desktop Layout/setup screen was captured and visually opened at `.scratch/ui-dev/layout-baseline.png`. No page errors were observed in that flow. Native in-app browser automation was unavailable on this Linux host, so isolated Chromium via bundled Playwright was used. This is startup evidence, not completed critique/audit or code acceptance. Development does not package the offline service worker and displays its missing-service-worker notice; no offline acceptance claim follows. Full affected checks, final graph binding and one fresh Sol medium acceptance are queued with root. No broader visual redesign has been accepted or implemented yet.

The private watcher uses `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`, `BOARDSTUDIO_CHECK_JOBS=1`, `BINARYEN_CORES=4`, and a one-browser cap. `.scratch/ui-dev/runtime.json` is the canonical current process receipt for its PTY session, PID/process group, target and ports; older server log files describe prior watcher processes. Keep using the existing 8182 proxy / 8183 Dioxus ports.

### Reused interface crosswalk

| Existing surface | Reuse in this batch | Ownership preserved |
| --- | --- | --- |
| `LayoutCommandPill` | Inline selection-context actions and the visible snap increment/toggle | Existing selection, menu-owner, and snap-intent callbacks |
| `LayoutTransformToolbar` | Full Transform menu remains the advanced fallback | Existing `LayoutTransformMenuMount` availability and tool callbacks |
| `LayoutSnapMenu` | Advanced increment, geometry, envelope-gap, and gap-value controls remain | Existing root-owned snap settings and intents |
| `activate_transform_tool` | Direct tools route through the established activation callback | Existing active-tool and selection-transition behavior |

First batch acceptance is limited to the command surface: direct actions stay hidden when pointer tools are not visible, tool labels expose their target scope, geometry snap is operable from the bar, and the existing menus retain their advanced routes. PCB continues to use the shared snap menu without Layout-only quick actions or snap-state decoration.

Luna 6 medium read-only source research identified the following candidate first batch, subject to the rendered baseline and user direction:

- `web/src/presentation.rs`: private `TabIcon`, shell navigation and project controls.
- `web/crates/ui-shared/src/panels.rs` and `lib.rs`, plus a new local icon module: reusable accessible icons and panel controls.
- `web/assets/m1.css`: existing theme, control-density and breakpoint rules; preserve geometry/status color distinctions.
- Narrow Solar asset provenance/license files and UI-local mounted accessibility tests as required by the accepted batch.

This is a proposed path set, not a writer claim. The existing **Create new keyboard** route in `web/crates/library/src/library.rs` supplies a real user-created project for baseline Layout/Case inspection without demo dependencies. Specific Layout command controls live in `web/crates/layout/src/objects/layout_toolbar.rs`; Case composition is in `web/src/presentation/case_workspace.rs` and existing Case presentation components. These remain later scopes until actual task review.

Solar's upstream distribution identifies the icon artwork as Solar Icons by 480 Design, CC BY 4.0, separately from its MIT wrapper code. Preserve source/version, exact assets and visible attribution when importing; see [upstream third-party terms](https://github.com/saoudi-h/solar-icons/blob/main/LICENSE-THIRD-PARTY). No icons have been imported. The research agent's query against another checkout alias is discarded as graph evidence; root's explicitly named integration index remains the only provisional alias until `boardstudio-2182` is bootstrapped.
