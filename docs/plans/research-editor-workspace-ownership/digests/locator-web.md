# Web locator digest

Fresh Scout; search only at ad9db4730; no source-body analysis or test execution. All paths relative to repository. These pointers select analyzer inputs, not implementation conclusions.

## Q1

- `web/src/presentation.rs:283 (WorkbenchApp) @ ad9db4730`: Runtime construction284, subscription334, reconciliation412, unsubscribe451-455, providers462; `web/src/presentation.rs:3273 (Editor) @ ad9db4730`: controller assembly, observer teardown5218, selection cleanup6170/6207.
- `web/crates/runtime/src/runtime.rs:455 (Runtime::new) @ ad9db4730`: constructor; subscribe1104/unsubscribe1107; Drop8013.
- `web/src/presentation/workspace_composition.rs:15 (WorkspaceObjectsInput) @ ad9db4730`: typed inputs and dispatch71/82/96/107.

## Q2

- `web/src/presentation/layout_workspace.rs:159 (use_layout_workspace_state) @ ad9db4730`: state, inputs31/44/68, renders358/425/845. Editor controller hits3455-3655; pointer surface9964.
- `web/src/presentation/pcb_workspace.rs:23 (CanvasInput) @ ad9db4730`: surface inputs/renders46/70/93, cancellation111-112. Editor controller hits3819.
- `web/src/presentation/parts_workspace.rs:23 (use_parts_workspace_state) @ ad9db4730`: state and renders106/124/134/148, admission87-94.
- `web/src/presentation/case_workspace.rs:39 (use_case_workspace_state) @ ad9db4730`: state, Admission131, scene182, tree/display actions693/850, render987/991/1019.
- `web/src/presentation/keymap_workspace.rs:87 (use_projection) @ ad9db4730`: projection/state15, render148/165/179/214; Editor controller hits3934/3964/3985.
- `web/src/presentation/keycaps_workspace.rs:20 (use_projection) @ ad9db4730`: projection, state51, retirement60, finding92, render258/285/299/338.
- `web/src/presentation/setup_workspace.rs:15 (use_setup_workspace_state) @ ad9db4730`: guide/instance state, methods118/143/158; Editor preview cleanup9023.
- `web/crates/ui-shared/src/canvas_navigation.rs:37 (use_canvas_navigation_state) @ ad9db4730`: state, cleanup55, mount125, pan151/174/190, wheel261.

## Q3

- `web/crates/pcb/src/pcb_wiring/controller.rs:936 (use_pcb_wiring_controller) @ ad9db4730`: query, controlled resolution960, teardown968; Editor consumers8840/9205.
- `web/crates/pcb/src/pcb_physical_setup/controller.rs:175 (use_controller) @ ad9db4730`: identity27, intent40, projection66, mount78, submit106.
- `web/crates/layout/src/part_placement.rs:432 (use_controller_placement) @ ad9db4730`: owner currentness152, stale preparation1101-1135, cancel1300, view-mode cancel1955.
- `web/crates/layout/src/layout_findings_state.rs:29 (use_layout_findings_state) @ ad9db4730`: focus63, owner123, current predicates144/179, destination214.
- `web/src/presentation.rs:1231 (perform_layout_finding_navigation) @ ad9db4730`: route; Editor pending fit6003, transaction6665, cancel6860, workspace effect6914.
- `web/src/presentation/keycaps_workspace.rs:92 (KeycapsWorkspaceState::focus_finding) @ ad9db4730`: finding identity.

## Q4

- `web/src/presentation/layout_draft_safety_tests.rs:1656 (matrix_hidden_draft_survives_inspector_park_workspace_switch_and_update) @ ad9db4730`: mounted draft proof definition, session retirement1712, admission1748, route closure1792.
- `web/src/model_decode_boundary_tests.rs:60 (refuses_hostile_wrl_at_the_application_decoder_without_poisoning_the_shared_module) @ ad9db4730`: packaged decoder regression definition.
- `web/crates/runtime/src/diagnostic_persistence.rs:288 (wrap_store) @ ad9db4730`: packaged persistence controls295, held-save646, failure698, window820.
- `web/src/presentation/case_workspace.rs:1582 (case_inspector_navigation_keeps_hooks_in_the_mounted_child) @ ad9db4730`: hook test; same-scope retirement1673.
- `web/crates/layout/src/part_placement.rs:3254 (production_hook_never_redirects_a_stale_board_or_project_owner) @ ad9db4730`: stale-completion test, suspended prepare3040.
- `web/crates/layout/src/layout_findings_state.rs:487 (mounted_back_enter_focuses_board_and_outline_inspectors_and_rejects_stale_scope) @ ad9db4730`: focus/currentness test.
- `web/crates/runtime/src/runtime.rs:11023 (restart_builds_a_fresh_engine_and_stale_replies_are_ignored) @ ad9db4730`: restart test.

## Q5 and negatives

Editor-lifetime wiring rationale comment at pcb_wiring/controller.rs934; setup shared-state module comment at setup_workspace.rs1; finding focus-order comment at layout_findings_state.rs209. No history conclusions. Case-insensitive shared.?plan|deferred.?focus|composition.?test found no matches in web/src, web/crates/runtime/src or web/tests at snapshot; located identifiers differ. First git ls-tree exclusion syntax failed, successful web listing followed. Eighteen strongest unique paths; other paths reused across questions.
