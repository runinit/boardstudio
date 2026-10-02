# Source checks for next Case and Export drafts

Read-only source check in `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, HEAD `47cf655baabde025628f15cae94e5b34e6dabaf4`; pinned React reference `5a472a9426e6e38993361da402cd4ec730feb369`. The continuation has no `.codegraph` index. The shared worktree had unrelated code, CSS and evidence changes present or appearing during this read-only review; none of those repository paths were written by this task.

## Canonical graph and existing authority

- `.scratch/dioxus-frontend-v1/tasks.json` is the 62-parent dependency authority. It calls F7.1 “Common viewer contract and renderer feasibility,” F7.2 “Authored Case body and stack editor,” and F8.1 “Complete Export workspace, exact output rows, readiness and return path.” F7.1 has no start or acceptance blockers; F7.2 and F8.1 start after INT.1 and have no additional parent acceptance joins. T1-01 is recorded `accepted` in `.scratch/dioxus-frontend-tranche-1/execution.json` and its issue file. Thus the F7.1, F7.2 and F8.1 candidates are currently start-eligible. The user-facing Case workspace is canonically F7.2; calling it F7.1 would mislabel the graph.
- `.scratch/dioxus-frontend-v1/workflows/F7.json` and `workflows/F8.json` preserve each parent task's exact start/acceptance joins. No parent status or dependency is altered by these drafts.
- `.scratch/dioxus-frontend-tranche-1/AUTHORITY.md` authorizes routine generated ticket publication under independent source/review safeguards. `ACCEPTANCE.md` requires integration through the public Dioxus app and the existing Session/provider, paired fixture/reference behavior, relevant error/scope/lifecycle states, accessibility/visual evidence, required checks and an RF handoff. `EXECUTION.md` makes `presentation.rs`, `runtime.rs`, global CSS/build and ledgers coordinator-owned; feature owners use private modules and provide bounded shared integration. No broad phase completion barrier was added.
- `.scratch/dioxus-frontend-v1/AGENT-ROUTING.md` routes F7.1 to Luna High with dedicated Astra review because it establishes a renderer boundary; F7.2 and F8.1 are Luna Medium with workflow-batch-eligible Astra review.

## F7.1 source path

- React common viewer and Case consumers: `app/src/ui/AssemblyViewer.tsx`, `app/src/ui/Workbench.tsx:1386`, `app/src/ui/useCaseWorkspace.tsx`, `app/src/ui/CaseInspectorPanel.tsx`.
- Dioxus page binary/callers: `web/src/main.rs`, `web/src/cad_presentation.rs`; current Case is a minimal mechanical-settings form and `CaseCanvas`, not the complete Case editor.
- Existing host wrappers: `web/src/renderer_host.rs` (`mount`, `update_scene`, `orbit`, `zoom`, `view`, `fit`, `dispose`). The wrapper currently lacks typed/private display-state, handle and pick operations.
- Existing WASM renderer exports: `renderer/src/wasm.rs` (`set_scene`, `set_state`, `fit`, `view`, `orbit`, `zoom`, `set_handles`, `pick`). Exported scene/camera/state operations already exist, so the current observed gap is host mapping and contract/placement proof, not CAD engine work.
- Crate boundary: `web/src/main.rs` declares the page `cad_presentation`, `presentation` and `runtime` modules in the binary; `web/src/lib.rs` separately exports `renderer_host`. A `pub(crate)` library method is not callable from the page binary. This matches existing RF-002/RF-012 and the F7.1 checkpoint in `EXECUTION.md`.

## F7.2 source path

- React authored Case body stack: `app/src/ui/CaseInspectorPanel.tsx` renders body list, empty state, body type, thickness/clearance/z/walls, mounting holes/bosses and gasket-channel fields; `app/src/ui/useCaseWorkspace.tsx` selects/creates bodies and emits `set-case` edits; `app/src/ui/Workbench.tsx:321–329` composes Case workspace inputs and distinguishes authored Case data from generated assembly. `app/src/ui/useCaseWorkspace.tsx:84–96` shows generated mode with authored bodies retained but the editor hidden; when the mechanical configuration belongs to another board it shows a mismatch panel and “Show configured board” beside the selected board’s authored editor. F7.4 owns mechanical configuration and disable behavior.
- Domain/session path: `core/src/model.rs` defines `CaseBody` and `EditOperation::SetCase`; `core/src/lib.rs:1186` applies `SetCase`; `application/src/session.rs` owns accepted snapshots and edit/history behavior.
- Dioxus baseline: `web/src/cad_presentation.rs` currently projects the selected captured Case document, an Add case settings action, bottom-thickness input, Generate/Cancel and a Canvas. It does not render authored Case bodies or their editor controls. `web/src/case_settings.rs` initializes/updates `MechanicalConfiguration` only; that is F7.4's generated mechanical configuration, not a replacement for `CaseBody` editing.
- F7.7 owns physical-instance selection/projection joins; F7.6 owns readiness/generation/STEP; F7.4 owns mechanical config. These are not preconditions for saved-fixture body editing in F7.2's canonical graph.

## F8.1 source path

- React Export row policy and navigation: `app/src/ui/Workbench.tsx:1049–1072` defines the seven output rows, readiness details/reasons, optional generated package, Review wiring/Review case, and portable copy checkbox/button; `:541–545` routes Escape; `:603–609` returns to `lastDesignMode` while preserving board context. `app/src/main.tsx:26,48,173` owns portable-copy preference wiring; the Export view labels this section “Portable project,” describes keeping an editable copy of the whole project including all boards, and offers “Save .boardstudio project.”
- Export inputs and row scoping: `app/src/exports/context.ts` and `app/src/ui/Workbench.tsx` derive selected board, wire/readiness, authored/generated Case state; generated readiness depends on current preview/generation/assembly inputs in `app/src/ui/useCaseWorkspace.tsx:31–36`; mechanical mismatch/generated presentation spans `app/src/ui/useCaseWorkspace.tsx:84–96`. Provider execution and delivery are split across `app/src/createProjectExporter.ts`, `app/src/exports/*`, `app/src/ExportClient.ts` and the workers; these are outside F8.1.
- Dioxus baseline: `web/src/presentation.rs:401–418` renders only “Export archive” and generic “Export STEP”. Workspace tab and routing are in the same shared file (`:195`, `:958–961`). There is no prior-workspace/escape behavior in that export component. `web/src/runtime.rs` already owns archive/STEP operations and a single Session; F8.1 should present the route without claiming the later typed provider/delivery behavior.
- Private file boundary: a feature-owned component can live in a dedicated `web/src/presentation/` module; route registration, shared navigation signal, `web/src/presentation.rs`, `web/src/runtime.rs`, `web/assets/m1.css` and build wiring stay coordinator-owned under `EXECUTION.md`. The coordinator supplies the minimal callback seam and performs the serial integration.

## Refactoring handoff

No new refactoring takeaway was observed during this planning-only source review. The known renderer wrapper gaps are already recorded in RF-002/RF-012; the page/library crate split is already an explicit F7.1 checkpoint. Missing UI/provider integration alone does not establish a new design defect.

This work read source and planning documents only. It changed no repository/source/build/browser/Git state. Draft outputs are confined to this directory under `/tmp`.
