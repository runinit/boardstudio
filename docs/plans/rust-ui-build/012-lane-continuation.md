# 012 safe handoff — 2026-10-22

Paused at the user's request for coordinator-owned T3 delegation. This is a WIP
continuation record, **not closure or landing approval**. Preserve this checkout,
cache and evidence. Resume by merging the checkpoint and latest local feature in
the coordinator's new explicitly bound isolated child checkout; do not replay the
implementation. No review, new agent/thread, or next build was started for handoff.

## Pin, binding and inventory

- Original checkout: `/home/chris/.t3/worktrees/ts-boardstudio2/lane-012-command-palette`.
- T3 binding freshly verified: attached, branch `lane/012-command-palette`, project
  root `/home/chris/01_Projects/ts-boardstudio2`.
- Original thread: `mcp:287ab835-3b1a-497d-96e7-52c21bc33c9a`.
- Coordinator: `9b186519-9e71-4e26-a24b-6587e94c69e4`.
- Initial clean base: `1620af409354adbec350b2fb8b8e03f70ca96585`.
- Claim-before-implementation: `b76a462ae` (`docs: claim rust-ui ticket 012 command palette`).
- Main implementation WIP: `6cf00b6e7`.
- Local 013 feature merge: `2f95c6b2d2c844a08476ca842c6caccd0cd487e3`.
- **Source/harness checkpoint HEAD: `0d6bafb65bd42e5cd3cb6a316709cf6accfdb9f8`**.
  The next documentation-only commit contains this handoff; its exact HEAD is
  reported to the coordinator. `git log -1` identifies it without source replay.
- Latest observed local `feat/rust-ui-migration`:
  `2de58e53aa5ca8a10d0223c2f32278ec42e3c623` (013 resolved/landed).
- Ticket `issues/012-command-palette.md` remains **claimed**, unchecked, no Outcome.
- Before checkpoint: only five tracked dirty files (all owned):
  `scripts/test-command-palette.py`, `web/crates/library/src/library.rs`,
  `web/crates/ui-shared/src/controls.rs`, `web/src/presentation.rs`,
  `web/src/presentation/command_palette.rs`; all committed in `0d6bafb65`.
- After that checkpoint: tracked dirty/untracked nonignored inventory empty.
  Only this continuation file was subsequently added for the handoff commit.
- Ignored state preserved: `.claude/`, `.evidence/`, `.memsearch/`, `cad/.cache/`,
  `cad/wasm/pkg/`, `renderer/pkg/`, `target`, `web/target/`, Python caches,
  generated web provider assets, `tailwind.css`, imported-module assets.
  No targets/source copies/manifests were placed under `.evidence/`.

## Processes and target

- `target` resolves to `/home/chris/.cache/boardstudio-target/012-command-palette`.
  Existing protocol `.claude/settings.json`, `cad/.cache`, `cad/wasm/pkg` were
  copied when absent during setup. Do not delete or repurpose the original cache.
- Final in-flight command was production `test-command-palette.py`, exec session
  `32840`: **completed exit 0**, then checkpointed. Its agent-browser session closed.
- At handoff no Cargo, wasm-pack, build-web or palette test command was running.
- Static server intentionally retained: PID **2849824**, parent **2849813**,
  exec session **58053**:
  `python3 scripts/serve-web.py web/target/site 4212 > .evidence/012/server.log 2>&1`.
  URL `http://127.0.0.1:4212/boardstudio/`. It serves candidate-build-3; root route
  was not freshly repackaged. Do not restart unrelated services.
- No lane agents/threads/watchers were spawned. Debug browser sessions
  `palette-012-debug`, `palette-012-snap`, `palette-012-keys` were closed.
  Other lane browser sessions, including `dock-011-page`, are not ours.
- New child must use its own protocol target/settings/caches and never run
  overlapping Cargo/wasm-pack against a target. Respect project jobs=12;
  `BOARDSTUDIO_CHECK_JOBS=1` was used. No concurrency/debug override flags.
- Gortex is uninstalled: no calls, restoration or requirements. `.codegraph/`
  absent, so CodeGraph skipped. Historical workflow docs are not instructions.

## Preserved implementation and ownership

Owned source inventory (plus this record and ticket claim):

```
scripts/test-command-palette.py
scripts/test-header.py
web/assets/command-palette.css
web/crates/case/src/cad_presentation.rs
web/crates/layout/src/objects/layout_toolbar.rs
web/crates/layout/src/objects/layout_transform_toolbar.rs
web/crates/library/src/library.rs
web/crates/library/src/library/header_actions.rs
web/crates/ui-shared/src/controls.rs
web/crates/ui-shared/src/ui/command.rs
web/crates/ui-shared/src/ui/copy_rules_tests.rs
web/crates/ui-shared/src/ui/mod.rs
web/src/presentation.rs
web/src/presentation/command_palette.rs
web/src/presentation/export_workspace.rs
web/src/presentation/header.rs
web/src/presentation/workbench_shortcuts.rs
web/src/presentation/zmk_firmware_export.rs
```

Read actual ticket, map shared contract/lane protocol, linked command-palette
spec/decision, migration spec/mapping/theme/accessibility/inventory/branch plan.
Used Ponytail and Dioxus skills. Keep scope narrow, preserve domain callbacks.

- Palette owns Dialog/Combobox query, focus and deferred command handoff. App
  palette hooks live in **WorkbenchApp** after 013's tiny NotificationProvider
  wrapper. Owned document listener uses Dioxus `use_callback` runtime and drops
  with owner. Ctrl/Cmd+K precedes history; Escape closes palette first.
- Approved shared hunks: `mod command_palette`, WorkbenchApp state/shortcut,
  Header `on_search`, sibling palette render after Header; header callback,
  Search and Kbd hints. No whole shared-block rewrites.
- New local registry honestly excluded from upstream COPIES provenance:
  `Command { group: &'static str, label: String, reason: Option<String>,
  hint: Option<&'static str>, on_select: EventHandler<()> }`, private-map
  `CommandRegistry`, `use_command_registry() -> CommandRegistry`,
  `use_commands(Vec<Command>)`, and public `CommandRegistry::commands()`.
  Public contract explicitly approved for implementation, still closure scope.
  One registration per ScopeId documented/asserted. Effect uses peek to avoid
  feedback; drop unregisters. Internal Rc owner token plus index and rendered
  command equality reject stale reordered/remounted/disabled activation.
  No bus, public IDs/options/persistence, or widened private visibility.
- Owner-local commands reuse real callbacks: Layout tools/disclosures, Case
  Update preview/Cancel, Library header tools, Export row callbacks, editor
  select_tree/fit/zoom. Objects use domain outline-generation guard. Object
  catalog now memoized by accepted source/token/scope/generation/workspace;
  hidden palette does not clone registry commands on canvas updates.
- Library approved seam: public `use_project_tools()` once WorkbenchApp,
  private request Signal<Option<ProjectTool>>, existing ProjectBrowserState.
  Existing Library consumes once after mount, opens existing rename/focus or
  demo chooser. Rename carries project id and now validates fresh accepted id
  before using captured current project. Draft validation/blur commit unchanged.
- Export hooks in ExportRowView before return, equivalent production hook in
  ZmkFirmwareExportRow. Native callback/provider/reason ownership unchanged;
  preserve 013 test-only provider/query/bridge hunks in same file.
- Approved controlled Snap seam in `BoardAccordionSection`: optional
  `ReadSignal<Option<bool>>` and `EventHandler<bool>`. None preserves existing
  uncontrolled Accordion branch; Some uses public controlled Collapsible
  primitives, same id/name/classes/heading/content. LayoutSnapDock owns one
  signal shared by native trigger and palette. All wrapper callers inspected
  (LayoutSnapDock and canvas_layers). No primitive private API widening.

## Exact pending seams and restrictions

- **011 not landed/integrated at handoff. Wait for local feature landing.**
  Its thread is `mcp:12c4a7b3-edbf-47b9-9f2f-0907aa3279c4`.
  It owns presentation panels import/tracks/Objects+Inspector DockLayout/card
  wrapping, old scrim/sheet removal, possible Case compact-focus close handler;
  panels.rs private helpers; authoritative CompactPanelState App context and
  Editor-local PanelSettings stay where they are. No workspace_composition edits.
- Approved post011 hunk: append View Show/hide Objects and Inspector commands
  to existing Editor `palette_commands` just before `use_commands`. Capture
  existing compact state and side settings; dispatch reads CURRENT viewport
  and settings. Compact width<980 toggles chosen side and closes opposite;
  desktop only Pinned<->Collapsed preserves width. Same native control logic,
  no panels.rs edits/helper exposure/new state. Names must match landed controls.
- 011 owns additive Button ARIA props (`aria_expanded`, `aria_controls`,
  `aria_haspopup`) and Sheet IDs. After landing reuse those for Header Search
  popup ARIA; current Search does not yet have those props. Do not compete in Button.
- 013 merged intact: App->NotificationProvider->private WorkbenchApp,
  notifications module/runtime bridge/banner suppression, runtime typed queue,
  exact lease/export_delivery_errors fixes, ZMK TESTONLY adapters, test-only
  ExportPanel banner adapter. Preserve all when merging latest feature.
- Coordinator ticket012 dirty-cleanup docs remain untouched.
- Latest approved Snap fix is **controlled CollapsibleTrigger Space keydown
  stop_propagation ONLY** (Dioxus and existing WebEventExt raw boundary). No
  preventDefault, canvas_interaction changes, broad interception or private API.
  **Outstanding known issue:** current child stop prevents Space reaching the
  wrapper's existing root `onkeydown` listener. Preserve caller composition by
  forwarding that caller exactly once for Space before stopping, and test it;
  do not double-call other keys. No new fix was started after handoff request.

## Executed evidence (original checkout `.evidence/012/`)

All paths below are relative to the original checkout. Evidence is ignored and
must remain there; new child can read it by absolute path. Supplied/earlier
receipts are not fresh final acceptance.

| Command/check | Actual result and receipt |
| --- | --- |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build` on initial base | PASS 445.9s, `red-build.log`; baseline provider/full build, not final source |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4212/boardstudio/ --evidence .evidence/012/red` before behavior edits | Actual browser RED: Search click succeeds, expected dialog absent, wait times out25s; `browser-red.log` |
| Typecheck attempts | `typecheck-1.log` through `typecheck-6.log`; compile diagnostics, not behavioral REDs: actual Accordion has no controlled open API, Tooltip as callback/type corrections etc. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only ui-shared` | FAILED first at new registry fixture `command.rs:163` unwrap before mount; `ui-shared-browser.log`. Later panic-hook/timeouts cascaded. Fixed initial wait, broad package not rerun yet. |
| `wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib -- ui::command::tests::` | PASS 2/2, 66 filtered, `command-tests-2.log`; registry refresh/reason/unregister/reorder/remount and controlled/uncontrolled disclosure. Before latest Space source fix/013 merge. |
| `python3 scripts/build-web.py --fast --skip-providers --routes subpath` | PASS `candidate-build.log`62.77s, `candidate-build-2.log`30.81s, `candidate-build-3.log`56.05s. Third contains all checkpoint source and merged013. Reused baseline providers; subpath only, not final both-route/provider clearance. |
| Production browser candidate1 | `browser-green-1.log` failed shortcut reopen after settings; native Escape callback also exposed outside-runtime panic (`browser-green-1-console.log`). Corrected owned Dioxus callback and dispatch after actual Dialog content unmount + one DOM-flush event turn, no animation-delay guess. |
| Production browser candidate2 | `browser-green-2.log` failed Snap Enter because agent-browser focus role string was a no-op. Corrected driver to snapshot role/name unique ref and assert actual focus. Native Enter then works. |
| Production browser candidate3 | `browser-green-3.log` real trusted Space failed. Existing Layout window Space-pan listener turns defaultPrevented false at document into true at window for Snap button. Correlated trace retained in `snap-space-red.log`. No canvas listener change. |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4212/boardstudio/ --evidence .evidence/012/green-4 > .evidence/012/browser-green-4.log 2>&1` | **PASS exit0**, completed at handoff. Role/name interactions, dialog/combobox ARIA/focus, filter/active descendant, settings dispatch/reopen, workspace/context hiding, Tab/ShiftTab, controlled Snap palette+Enter+trusted Space keyup with exactly one click/toggle per key, focus retained, underlying open state survives palette Escape, nonLibrary rename focus/blur commit/Escape cancel, real demo chooser/copy, object group presence, ZMK unavailable reason. |
| Formatting/diff | `cargo fmt --all` and Python py_compile previously passed; `git diff --check` fresh before source checkpoint PASS. No final full check after all remaining changes. |

Screenshots: `.evidence/012/green-4/palette-open.png` and
`palette-closed.png`. `server.log` records serving. No current first unresolved
test failure: green-4 succeeded. Earliest broad-package failure is the historical
fixed fixture unwrap; its broader rerun remains pending.

## Resume next, without implementation replay

1. Merge checkpoint + latest local feature in the new T3-bound child, inventory
   overlap, configure separate target/protocol caches before builds. Keep this
   lane intact. If011 still unlanded, continue independent checks only.
2. Preserve controlled Space caller `onkeydown` composition exactly once; add
   focused verification for callback counts/Enter/uncontrolled branch. Rebuild
   and real trusted-key rerun after that source change.
3. Prove canvas Space pan outside control and typing unchanged. Current green
   proves native Snap activation only; no held-Space drag receipt yet. Existing
   ui-model native Space-pan and ui-shared canvas_navigation mounted tests are
   useful affected checks but not a substitute for an actual browser claim.
   agent-browser CLI lacks separate keydown/up; Node built-in WebSocket exists,
   websocket-client does not. No CDP helper/dependency was added. Use minimal
   supported input route if needed; keep role/name interaction ownership.
4. Broaden actual object search to selecting each required kind with real
   Inspector/domain outcome; currently object presence only. Verify Layout
   tool callback/hints, Case action/reason, Export ready/disabled callbacks,
   themes/appearances/narrow accessibility where relevant. Do not claim the
   current production script covers all acceptance or all commands.
5. Add focused project request race/no-replay checks: fresh guard exists but
   normal rename/demo browser flow is the executed receipt, not project-switch
   race proof. Preserve normal draft/blur/cancel and no duplicate Library owner.
6. After011 lands add approved dock commands and Search ARIA; verify dispatch
   while viewport changes with palette open,979/980, opposite compact side,
   desktop modes/width persistence/reopen and same owner callbacks.
7. Re-run broader ui-shared browser after fixed fixture/latest source, affected
   library/layout/case/page checks and documented formatting/copy-rule/native
   checks as appropriate. One Cargo target build at a time. Fresh final build
   both routes/providers after latest feature merge; no next long build was
   started during this handoff.
8. Record honest ticket Outcome (including Accordion compile diagnostic vs
   actual trusted Space RED/cause/GREEN and listener composition correction).
   At clean final pin, request **ONE real Standards+Spec closure review through
   coordinator**, full API/owner source scope and receipts. No review started.
   HOLD landing until substantive approval. Resolve only after real closure.

Global full/quick, CAD worker.onerror, first-paint, screen-reader, human026 and
final-merge gates remain **open**. 013 runtime50/page5/production GREEN are
coordinator-reported receipts in lane013 evidence, not fresh012 global clearance.
No origin/dev pushes, local feature landing, reset/discard/history rewrite,
unrelated configs, API visibility widening or destructive cleanup authorized.
