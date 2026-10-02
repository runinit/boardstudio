use super::{
    InstanceSelection, active_board_scope_matches,
    pcb_wiring::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity},
};
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub(crate) struct ZmkFirmwareExportPanelInput {
    pub ready: bool,
    pub on_export: EventHandler<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ZmkExportActionOwner {
    ui_scope: Scope,
    plan_identity: WiringPlanIdentity,
    scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AcceptedDocumentIdentity {
    session_epoch: SessionEpoch,
    document_id: String,
    token: SnapshotToken,
    revision: u64,
    scene_revision: u64,
}

struct CurrentZmkExportState<'a> {
    workspace: &'a str,
    scope_generation: u64,
    runtime_scope: Option<&'a Scope>,
    board_scope_is_current: bool,
    instance_is_current: bool,
    accepted: Option<&'a AcceptedDocumentIdentity>,
    executor_epoch: u64,
    resolution: &'a PcbWiringResolution,
}

fn export_action_is_current(
    owner: &ZmkExportActionOwner,
    state: &CurrentZmkExportState<'_>,
) -> bool {
    let plan = &owner.plan_identity;
    state.workspace == "Export"
        && state.scope_generation == owner.scope_generation
        && state.runtime_scope == Some(&owner.ui_scope)
        && state.board_scope_is_current
        && state.instance_is_current
        && state.executor_epoch == plan.executor_epoch
        && state.accepted.is_some_and(|accepted| {
            accepted.session_epoch == plan.scope.session_epoch
                && accepted.document_id == plan.scope.document_id
                && accepted.token == plan.token
                && accepted.revision == plan.revision
                && accepted.scene_revision == plan.revision
        })
        && ready_for_export(plan, state.resolution)
}

pub(crate) fn use_export_panel_input(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
    instance_selection: InstanceSelection,
) -> ZmkFirmwareExportPanelInput {
    let ready = source.as_ref().is_some_and(|source| {
        ready_for_export(&source.identity, &resolution())
            && runtime.electrical_preview_executor_epoch() == source.identity.executor_epoch
    });
    let on_export = use_callback({
        let runtime = runtime.clone();
        let owner = source.as_ref().map(|source| ZmkExportActionOwner {
            ui_scope: source.ui_scope.clone(),
            plan_identity: source.identity.clone(),
            scope_generation: source.scope_generation,
        });
        move |()| {
            let Some(owner) = owner.as_ref() else {
                return;
            };
            let model = runtime.model();
            let accepted = model
                .accepted
                .as_ref()
                .map(|snapshot| AcceptedDocumentIdentity {
                    session_epoch: snapshot.session_epoch,
                    document_id: snapshot.document.id.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                    scene_revision: snapshot.scene.revision,
                });
            if !export_action_is_current(
                owner,
                &CurrentZmkExportState {
                    workspace: workspace(),
                    scope_generation: scope_generation(),
                    runtime_scope: runtime.scope().as_ref(),
                    board_scope_is_current: active_board_scope_matches(&model, &owner.ui_scope),
                    instance_is_current: instance_selection.is_current(&model),
                    accepted: accepted.as_ref(),
                    executor_epoch: runtime.electrical_preview_executor_epoch(),
                    resolution: &resolution(),
                },
            ) {
                return;
            }
            runtime.export_firmware();
        }
    });
    ZmkFirmwareExportPanelInput { ready, on_export }
}

pub(crate) fn ready_for_export(
    identity: &WiringPlanIdentity,
    resolution: &PcbWiringResolution,
) -> bool {
    let PcbWiringResolution::Current {
        identity: resolved_identity,
        plan,
    } = resolution
    else {
        return false;
    };

    resolved_identity == identity
        && plan.revision == identity.revision
        && plan.board_id.as_deref() == Some(identity.scope.board_id.as_str())
        && plan.instance_id.is_none()
        && !plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
        && plan.controller_part_id.is_some()
        && !plan.assignments.is_empty()
}

#[component]
pub(crate) fn ZmkFirmwareExportRow(ready: bool, on_export: EventHandler<()>) -> Element {
    rsx! {
        div { class: "m1-export-row",
            span {
                class: if ready { "m1-export-ready-dot is-ready" } else { "m1-export-ready-dot" },
                "aria-hidden": "true",
            }
            span { class: "m1-export-row-copy",
                strong { "ZMK firmware" }
                small { "ZMK v0.3.0 configuration and editable starter keymap" }
                if !ready {
                    small { class: "m1-export-reason", "Review the layout and resolve controller wiring in PCB." }
                }
            }
            button {
                class: "m1-export-row-action",
                r#type: "button",
                aria_label: "Export ZMK firmware",
                disabled: !ready,
                onclick: move |_| on_export.call(()),
                if ready { "Export" } else { "Needs work" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ExportPanel, InstanceSelection, RuntimeReportBanner};
    use super::*;
    use crate::runtime::firmware_export_test_support as runtime_test;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::electrical::{ElectricalMode, ElectricalPlan};
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[component]
    fn row_render_fixture() -> Element {
        let no_op = use_callback(|()| {});
        rsx! {
            style { {include_str!("../../assets/m1.css")} }
            ZmkFirmwareExportRow { ready: false, on_export: no_op }
            ZmkFirmwareExportRow { ready: true, on_export: no_op }
        }
    }

    #[derive(Clone)]
    struct ProductionExportProbe {
        runtime: Rc<Runtime>,
        source: PcbWiringSource,
        resolution: PcbWiringResolution,
    }

    #[component]
    fn production_export_panel_fixture() -> Element {
        let probe = use_context::<ProductionExportProbe>();
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        let _ = version();
        let runtime = probe.runtime.clone();
        let version_cell = Rc::new(RefCell::new(version));
        use_hook(move || {
            let version_cell = version_cell.clone();
            runtime.subscribe(Rc::new(move || {
                let mut version = version_cell.borrow_mut();
                let next = *version.peek() + 1;
                version.set(next);
            }));
        });
        let workspace = use_signal(|| "Export");
        let generation = use_signal(|| 1u64);
        let resolution = use_signal(|| probe.resolution.clone());
        let instance_selection = InstanceSelection(use_signal(|| None));
        let firmware = use_export_panel_input(
            probe.runtime.clone(),
            workspace,
            generation,
            Some(probe.source.clone()),
            resolution,
            instance_selection,
        );
        rsx! {
            style { {include_str!("../../assets/m1.css")} }
            RuntimeReportBanner {}
            ExportPanel { zmk_firmware: Some(firmware) }
        }
    }

    fn mount_production_export_panel(probe: ProductionExportProbe) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("zmk-production-export-panel-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(production_export_panel_fixture);
        dom.provide_root_context(probe.runtime.clone());
        dom.provide_root_context(probe);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
    }

    fn production_probe(
        runtime: Rc<Runtime>,
        accepted: &boardstudio_application::AcceptedSnapshot,
        scope: &Scope,
        executor_epoch: u64,
        ready: bool,
    ) -> ProductionExportProbe {
        let source = PcbWiringSource::new(accepted, scope, scope, None, executor_epoch, 1)
            .expect("fixture has an accepted board-scoped source");
        let resolution = if ready {
            let identity = source.identity.clone();
            PcbWiringResolution::Current {
                identity: identity.clone(),
                plan: Rc::new(plan(&identity)),
            }
        } else {
            PcbWiringResolution::Idle
        };
        ProductionExportProbe {
            runtime,
            source,
            resolution,
        }
    }

    async fn browser_tick() {
        gloo_timers::future::TimeoutFuture::new(25).await;
    }

    #[wasm_bindgen_test]
    async fn mounted_export_panel_dispatches_failure_alert_and_successful_retry_from_same_row() {
        let runtime = runtime_test::new_runtime();
        let (session, accepted, scope) = runtime_test::opened_session();
        let failing = runtime_test::ControlledExecutor::failing(runtime_test::Stage::Generation);
        let failing: Rc<dyn crate::runtime::FirmwareExportExecutor> = failing;
        runtime_test::configure_runtime(
            &runtime,
            session,
            accepted.clone(),
            scope.clone(),
            failing,
            boardstudio_application::ExecutorEpoch(11),
            100,
        );
        let before = runtime_test::session_model(&runtime);
        let root = mount_production_export_panel(production_probe(
            runtime.clone(),
            &accepted,
            &scope,
            11,
            true,
        ));
        browser_tick().await;

        let headings = root.query_selector_all("h2").unwrap();
        assert_eq!(headings.length(), 2);
        assert_eq!(
            headings.item(0).unwrap().text_content().as_deref(),
            Some("Design files")
        );
        assert_eq!(
            headings.item(1).unwrap().text_content().as_deref(),
            Some("Portable project")
        );
        let row = root.query_selector(".m1-export-row").unwrap().unwrap();
        assert_eq!(
            row.query_selector("strong")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("ZMK firmware")
        );
        assert_eq!(
            row.query_selector("small")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("ZMK v0.3.0 configuration and editable starter keymap")
        );
        assert!(
            row.query_selector(".m1-export-ready-dot.is-ready")
                .unwrap()
                .is_some()
        );
        let button = row
            .query_selector("button[aria-label='Export ZMK firmware']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        assert_eq!(button.text_content().as_deref(), Some("Export"));
        button.click();
        browser_tick().await;

        assert!(matches!(
            runtime_test::take_events(&runtime).as_slice(),
            [boardstudio_application::Event::StartExport { scope: event_scope, .. }]
                if event_scope == &scope
        ));
        let effects = runtime_test::take_effects(&runtime);
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(
                    effect,
                    boardstudio_application::Effect::RunExport { .. }
                ))
                .count(),
            1,
            "the mounted hook invokes the existing firmware export operation"
        );
        runtime_test::run_effects(&runtime, effects).await;
        browser_tick().await;

        let alert = root
            .query_selector("[role='alert']")
            .unwrap()
            .unwrap_or_else(|| {
                panic!(
                    "mounted report alert missing; runtime status={:?}, DOM={}",
                    runtime.status(),
                    root.inner_html()
                )
            });
        assert_eq!(
            alert.text_content().as_deref(),
            Some("Firmware generation failed: injected firmware-generation failure")
        );
        assert_eq!(runtime_test::session_model(&runtime), before);
        assert!(!runtime_test::has_artifacts(&runtime));
        assert!(runtime_test::take_deliveries(&runtime).is_empty());
        assert_eq!(runtime_test::take_events(&runtime).len(), 0);
        let row_after_failure = root.query_selector(".m1-export-row").unwrap().unwrap();
        assert!(
            row.is_same_node(row_after_failure.dyn_ref::<web_sys::Node>()),
            "failure keeps the same row mounted for retry"
        );

        runtime_test::replace_executor(
            &runtime,
            runtime_test::ControlledExecutor::succeeding(),
            boardstudio_application::ExecutorEpoch(11),
        );
        button.click();
        browser_tick().await;
        assert!(root.query_selector("[role='alert']").unwrap().is_none());
        let retry_events = runtime_test::take_events(&runtime);
        assert_eq!(retry_events.len(), 1);
        assert!(matches!(
            retry_events.as_slice(),
            [boardstudio_application::Event::StartExport { .. }]
        ));
        let retry_effects = runtime_test::take_effects(&runtime);
        runtime_test::run_effects(&runtime, retry_effects).await;
        browser_tick().await;

        let status = root.query_selector("[role='status']").unwrap().unwrap();
        assert_eq!(status.text_content().as_deref(), Some("Saved locally."));
        let deliveries = runtime_test::take_deliveries(&runtime);
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].filename, "ZMK export test-zmk.zip");
        assert_eq!(deliveries[0].media_type.as_deref(), Some("application/zip"));
        assert_eq!(runtime_test::session_model(&runtime), before);
        assert!(runtime_test::take_events(&runtime).is_empty());
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_unavailable_export_row_is_disabled_and_never_dispatches() {
        let runtime = runtime_test::new_runtime();
        let (session, accepted, scope) = runtime_test::opened_session();
        let executor = runtime_test::ControlledExecutor::succeeding();
        let executor: Rc<dyn crate::runtime::FirmwareExportExecutor> = executor;
        runtime_test::configure_runtime(
            &runtime,
            session,
            accepted.clone(),
            scope.clone(),
            executor,
            boardstudio_application::ExecutorEpoch(11),
            200,
        );
        let root = mount_production_export_panel(production_probe(
            runtime.clone(),
            &accepted,
            &scope,
            11,
            false,
        ));
        browser_tick().await;
        let button = root
            .query_selector("button[aria-label='Export ZMK firmware']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        assert!(button.has_attribute("disabled"));
        assert_eq!(button.text_content().as_deref(), Some("Needs work"));
        assert_eq!(
            root.query_selector(".m1-export-reason")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Review the layout and resolve controller wiring in PCB.")
        );
        button.click();
        browser_tick().await;
        assert!(runtime_test::take_events(&runtime).is_empty());
        assert!(runtime_test::take_effects(&runtime).is_empty());
        assert!(!runtime_test::has_artifacts(&runtime));
        root.remove();
    }

    fn mount_row_fixture() -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(previous) = document.get_element_by_id("zmk-export-row-test-root") {
            previous.remove();
        }
        let root = document.create_element("div").unwrap();
        root.set_id("zmk-export-row-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(row_render_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
    }

    fn identity() -> WiringPlanIdentity {
        WiringPlanIdentity {
            scope: Scope {
                session_epoch: SessionEpoch(4),
                document_id: "doc-a".into(),
                board_id: "board-a".into(),
                instance_id: None,
            },
            token: SnapshotToken(8),
            revision: 3,
            executor_epoch: 2,
        }
    }

    fn plan(identity: &WiringPlanIdentity) -> ElectricalPlan {
        ElectricalPlan {
            instance_id: None,
            jumpers: Vec::new(),
            module_aliases: Default::default(),
            mode: ElectricalMode::Matrix,
            assignments: vec![boardstudio_core::electrical::ElectricalAssignment {
                key_id: "key-1".into(),
                matrix_id: "matrix-1".into(),
                row: 0,
                column: 0,
                row_pin: "R0".into(),
                column_pin: "C0".into(),
                locked: false,
                row_firmware_gpio: None,
                column_firmware_gpio: None,
                direct_gpio: None,
            }],
            row_pins: vec!["R0".into()],
            column_pins: vec!["C0".into()],
            diagnostics: Vec::new(),
            fingerprint: "plan".into(),
            board_id: Some(identity.scope.board_id.clone()),
            controller_part_id: Some("controller-1".into()),
            revision: identity.revision,
            controller_profile: None,
            free_pins: Vec::new(),
            nets: Vec::new(),
            diode_direction: "column2row".into(),
            peripherals: Vec::new(),
            peripheral_pins: Default::default(),
            peripheral_terminals: Default::default(),
        }
    }

    #[wasm_bindgen_test]
    fn only_a_current_ready_board_plan_with_controller_and_assignment_enables_export() {
        let identity = identity();
        let ready = PcbWiringResolution::Current {
            identity: identity.clone(),
            plan: Rc::new(plan(&identity)),
        };
        assert!(ready_for_export(&identity, &ready));

        assert!(!ready_for_export(&identity, &PcbWiringResolution::Idle));
        assert!(!ready_for_export(
            &identity,
            &PcbWiringResolution::Pending {
                identity: identity.clone(),
            }
        ));
        assert!(!ready_for_export(
            &identity,
            &PcbWiringResolution::Failed {
                identity: identity.clone(),
                message: "worker failed".into(),
            }
        ));

        let wrong_owner = WiringPlanIdentity {
            token: SnapshotToken(9),
            ..identity.clone()
        };
        assert!(!ready_for_export(
            &identity,
            &PcbWiringResolution::Current {
                identity: wrong_owner.clone(),
                plan: Rc::new(plan(&wrong_owner)),
            }
        ));

        let mut no_controller = plan(&identity);
        no_controller.controller_part_id = None;
        assert!(!ready_for_export(
            &identity,
            &PcbWiringResolution::Current {
                identity: identity.clone(),
                plan: Rc::new(no_controller),
            }
        ));

        let mut no_assignments = plan(&identity);
        no_assignments.assignments.clear();
        assert!(!ready_for_export(
            &identity,
            &PcbWiringResolution::Current {
                identity: identity.clone(),
                plan: Rc::new(no_assignments),
            }
        ));

        let mut diagnostics = plan(&identity);
        diagnostics
            .diagnostics
            .push(boardstudio_core::electrical::ElectricalDiagnostic {
                code: "missing-controller".into(),
                severity: "error".into(),
                message: "not ready".into(),
                key_id: None,
            });
        assert!(!ready_for_export(
            &identity,
            &PcbWiringResolution::Current {
                identity: identity.clone(),
                plan: Rc::new(diagnostics),
            }
        ));
    }

    #[wasm_bindgen_test]
    fn export_action_requires_the_rendered_workspace_owner_and_accepted_plan() {
        let identity = identity();
        let ui_scope = Scope {
            instance_id: Some("main".into()),
            ..identity.scope.clone()
        };
        let owner = ZmkExportActionOwner {
            ui_scope: ui_scope.clone(),
            plan_identity: identity.clone(),
            scope_generation: 7,
        };
        let accepted = AcceptedDocumentIdentity {
            session_epoch: identity.scope.session_epoch,
            document_id: identity.scope.document_id.clone(),
            token: identity.token,
            revision: identity.revision,
            scene_revision: identity.revision,
        };
        let resolution = PcbWiringResolution::Current {
            identity: identity.clone(),
            plan: Rc::new(plan(&identity)),
        };
        let admit = |workspace,
                     generation,
                     runtime_scope,
                     board,
                     instance,
                     accepted,
                     executor,
                     resolution| {
            export_action_is_current(
                &owner,
                &CurrentZmkExportState {
                    workspace,
                    scope_generation: generation,
                    runtime_scope,
                    board_scope_is_current: board,
                    instance_is_current: instance,
                    accepted,
                    executor_epoch: executor,
                    resolution,
                },
            )
        };

        assert!(admit(
            "Export",
            7,
            Some(&ui_scope),
            true,
            true,
            Some(&accepted),
            identity.executor_epoch,
            &resolution,
        ));
        assert!(!admit(
            "Keymap",
            7,
            Some(&ui_scope),
            true,
            true,
            Some(&accepted),
            identity.executor_epoch,
            &resolution,
        ));
        assert!(!admit(
            "Export",
            8,
            Some(&ui_scope),
            true,
            true,
            Some(&accepted),
            identity.executor_epoch,
            &resolution,
        ));
        let other_board = Scope {
            board_id: "other-board".into(),
            ..ui_scope.clone()
        };
        assert!(!admit(
            "Export",
            7,
            Some(&other_board),
            true,
            true,
            Some(&accepted),
            identity.executor_epoch,
            &resolution,
        ));
        assert!(!admit(
            "Export",
            7,
            Some(&ui_scope),
            true,
            false,
            Some(&accepted),
            identity.executor_epoch,
            &resolution,
        ));
        let mut stale = accepted.clone();
        stale.revision += 1;
        assert!(!admit(
            "Export",
            7,
            Some(&ui_scope),
            true,
            true,
            Some(&stale),
            identity.executor_epoch,
            &resolution,
        ));
        assert!(!admit(
            "Export",
            7,
            Some(&ui_scope),
            true,
            true,
            Some(&accepted),
            identity.executor_epoch + 1,
            &resolution,
        ));
    }

    #[wasm_bindgen_test]
    async fn row_renders_reference_copy_accessible_name_and_ready_marker() {
        let root = mount_row_fixture();
        gloo_timers::future::TimeoutFuture::new(40).await;
        let document = web_sys::window().unwrap().document().unwrap();
        let rows = document
            .query_selector_all("#zmk-export-row-test-root .m1-export-row")
            .unwrap();
        assert_eq!(rows.length(), 2);

        let unavailable = rows
            .item(0)
            .unwrap()
            .dyn_into::<web_sys::Element>()
            .unwrap();
        assert_eq!(
            unavailable
                .query_selector("strong")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("ZMK firmware")
        );
        assert_eq!(
            unavailable
                .query_selector("button")
                .unwrap()
                .unwrap()
                .get_attribute("aria-label")
                .as_deref(),
            Some("Export ZMK firmware")
        );
        assert!(
            unavailable
                .query_selector("button")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );
        assert_eq!(
            unavailable
                .query_selector(".m1-export-reason")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Review the layout and resolve controller wiring in PCB.")
        );
        assert!(
            !unavailable
                .query_selector(".m1-export-ready-dot")
                .unwrap()
                .unwrap()
                .get_attribute("class")
                .unwrap_or_default()
                .contains("is-ready")
        );

        let ready = rows
            .item(1)
            .unwrap()
            .dyn_into::<web_sys::Element>()
            .unwrap();
        assert!(
            !ready
                .query_selector("button")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );
        assert_eq!(
            ready
                .query_selector("button")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Export")
        );
        assert!(
            ready
                .query_selector(".m1-export-ready-dot")
                .unwrap()
                .unwrap()
                .get_attribute("class")
                .unwrap_or_default()
                .contains("is-ready")
        );
        root.remove();
    }
}
