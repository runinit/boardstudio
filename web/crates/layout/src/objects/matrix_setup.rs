//! Private immediate-origin Matrix Setup form, shared by Layout and the project guide.
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
};
use crate::matrix_setup_operation::MatrixSetupPreset;
pub use crate::presentation::parts::MatrixPlacementSource;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::collections::HashSet;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupProjection {
    pub owner: MatrixSetupOwner,
    pub editable: bool,
    pub can_cancel: bool,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupCreateRequest {
    pub owner: MatrixSetupOwner,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
}

#[derive(Clone, PartialEq)]
pub struct MatrixSetupMount {
    pub projection: Option<MatrixSetupProjection>,
    pub can_open: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixPlacementOwner {
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementProjection {
    pub owner: MatrixPlacementOwner,
    pub matrix: boardstudio_core::model::Matrix,
    pub scene: boardstudio_core::model::MatrixScene,
    pub definitions: Vec<boardstudio_core::model::PartDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementMove {
    pub owner: MatrixPlacementOwner,
    pub center: boardstudio_core::model::Vec2,
}

#[derive(Clone, PartialEq)]
pub struct MatrixPlacementMount {
    pub cancel_owner: Option<MatrixPlacementOwner>,
    pub placement: Option<MatrixPlacementProjection>,
    pub busy: bool,
    pub error: Option<String>,
    pub on_place: EventHandler<MatrixPlacementSource>,
    pub on_move: EventHandler<MatrixPlacementMove>,
    pub on_commit: EventHandler<MatrixPlacementMove>,
    pub on_cancel: EventHandler<MatrixPlacementOwner>,
}

#[derive(Clone)]
pub struct MatrixPlacementInput {
    pub version: Signal<u64>,
    pub selected_context: Signal<Option<super::ScopedTreeContext>>,
    pub anchor_scope: Signal<Option<Scope>>,
    pub workspace: Signal<&'static str>,
    pub scope_generation: Signal<u64>,
    pub assembly_orientation: crate::presentation::parts::PartsAssemblyOrientation,
    pub canvas_interaction: crate::presentation::canvas_interaction::CanvasInteractionArbiter,
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixSetupProps {
    pub projection: MatrixSetupProjection,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[component]
pub fn MatrixSetup(props: MatrixSetupProps) -> Element {
    let rows = use_signal(|| "5".to_owned());
    let columns = use_signal(|| "6".to_owned());
    let row_slider = use_signal(|| Some(5.0));
    let column_slider = use_signal(|| Some(6.0));
    let family = use_signal(|| Some(HashSet::from([0usize])));
    let mounting = use_signal(|| Some(HashSet::from([0usize])));
    let lighting = use_signal(|| Some(HashSet::from([0usize])));
    let return_focus = use_signal(|| None::<web_sys::HtmlElement>);
    let rows_value = rows();
    let columns_value = columns();
    super::matrix_setup_preview::use_matrix_setup_preview(
        &props.projection.owner,
        &rows_value,
        &columns_value,
    );
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = matrix_setup_preset(choc, hotswap, rgb);
    let starting_values = use_hook(|| (rows_value.clone(), columns_value.clone(), selected_preset));
    let admission = try_use_context::<super::matrix_setup_controller::MatrixSetupAdmission>();
    let mut submitted_values = use_signal(|| None::<(MatrixSetupCreateRequest, String, String)>);
    let runtime = try_use_context::<std::rc::Rc<crate::runtime::Runtime>>();
    let guard = use_signal(|| None::<boardstudio_web_runtime::draft_owners::DraftOwnerGuard>);
    let project_scope = (
        props.projection.owner.scope.document_id.clone(),
        props.projection.owner.scope.session_epoch,
    );
    use_effect(use_reactive((&project_scope,), {
        let runtime = runtime.clone();
        let mut guard = guard;
        let admission = admission.clone();
        move |(project_scope,)| {
            let Some(runtime) = runtime.as_ref() else {
                return;
            };
            if let Some(old) = guard.write().take() {
                runtime.retire_draft_owner(old);
            }
            let baseline = starting_values.clone();
            let admission = admission.clone();
            guard.set(Some(runtime.register_draft_owner(
                boardstudio_web_runtime::draft_owners::DraftOwnerRegistration {
                    project_scope,
                    workspace: "Layout",
                    owner: "matrix-setup",
                    admission_key: "create",
                    area: "New matrix".into(),
                    remedy: "Create the matrix or cancel the setup".into(),
                    is_unresolved: std::rc::Rc::new(move || {
                        let preset = matrix_setup_preset(
                            family
                                .peek()
                                .as_ref()
                                .is_some_and(|choice| choice.contains(&1)),
                            mounting
                                .peek()
                                .as_ref()
                                .is_some_and(|choice| choice.contains(&1)),
                            lighting
                                .peek()
                                .as_ref()
                                .is_some_and(|choice| choice.contains(&1)),
                        );
                        let changed = *rows.peek() != baseline.0
                            || *columns.peek() != baseline.1
                            || preset != baseline.2;
                        let carried = admission.as_ref().is_some_and(|admission| {
                            submitted_values.peek().as_ref().is_some_and(
                                |(request, submitted_rows, submitted_columns)| {
                                    admission.carries(request)
                                        && *rows.peek() == *submitted_rows
                                        && *columns.peek() == *submitted_columns
                                        && preset == request.preset
                                },
                            )
                        });
                        changed && !carried
                    }),
                },
            )));
        }
    }));
    use_drop(move || {
        let mut guard = guard;
        if let Some(runtime) = runtime
            && let Some(old) = guard.write().take()
        {
            runtime.retire_draft_owner(old);
        }
    });
    let owner = props.projection.owner.clone();
    let feedback_error = props.projection.error.clone();
    let feedback_status = props.projection.status.clone();
    let create_handler = props.on_create;
    let submit_owner = owner.clone();
    let submit_rows = rows_value.clone();
    let submit_columns = columns_value.clone();
    let on_submit = move |event: FormEvent| {
        event.prevent_default();
        let Ok(rows) = submit_rows.trim().parse::<u32>() else {
            return;
        };
        let Ok(columns) = submit_columns.trim().parse::<u32>() else {
            return;
        };
        let request = MatrixSetupCreateRequest {
            owner: submit_owner.clone(),
            rows,
            columns,
            preset: selected_preset,
        };
        if admission
            .as_ref()
            .is_some_and(|admission| !admission.is_pending())
        {
            submitted_values.set(Some((
                request.clone(),
                submit_rows.clone(),
                submit_columns.clone(),
            )));
        }
        create_handler.call(request);
    };
    let cancel_handler = props.on_cancel;
    let cancel_owner = owner.clone();
    let on_cancel = move |_| {
        cancel_handler.call(cancel_owner.clone());
        let return_focus = return_focus();
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            if let Some(target) = return_focus.filter(|target| target.is_connected()) {
                let _ = target.focus();
            } else if let Some(target) = document
                .get_element_by_id("m1-setup-guide-heading")
                .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = target.focus();
            }
        });
    };
    let mut return_focus_on_mount = return_focus;

    rsx! {
        section {
            class: "absolute left-2 top-[100px] bottom-2 z-6 flex w-[min(340px,calc(100%-16px))] flex-col gap-2.5 overflow-auto overscroll-contain rounded-md border border-border-strong bg-card p-3 text-foreground shadow-md [&_form]:grid [&_form]:gap-2 [&_footer]:flex [&_footer]:justify-end [&_footer]:gap-2",
            aria_label: "New matrix",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                }
            },
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-[16px]!", h2 { "New matrix" } }
            form { onsubmit: on_submit,
                div {
                    id: "m1-matrix-rows-focus",
                    onmounted: move |_| async move {
                        if let Some(target) = web_sys::window()
                            .and_then(|window| window.document())
                            .and_then(|document| document.active_element())
                            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                        { return_focus_on_mount.set(Some(target)); }
                        if let Some(target) = web_sys::window()
                            .and_then(|window| window.document())
                            .and_then(|document| document.query_selector("#m1-matrix-rows-focus [role='slider'][aria-label='Rows']").ok().flatten())
                            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                        { let _ = target.focus(); }
                    },
                    MatrixDimensionSliders {
                        rows, columns, row_slider, column_slider,
                        rows_label: "Rows", columns_label: "Columns", editable: props.projection.editable,
                    }
                }
                MatrixAssemblyPicker { family, mounting, lighting, editable: props.projection.editable }
                MatrixPreview { rows: rows_value.clone(), columns: columns_value.clone() }
                if let Some(message) = feedback_error {
                    Alert { variant: AlertVariant::Destructive, AlertDescription { "{message}" } }
                }
                if let Some(message) = feedback_status {
                    div { role: "status", Callout { variant: CalloutVariant::Info, "{message}" } }
                }
                footer {
                    if props.projection.can_cancel {
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| on_cancel(owner.clone()), "Cancel" }
                    }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !props.projection.editable, "Create matrix" }
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod draft_safety_tests {
    use super::*;
    use crate::objects::mirrored_pair::draft_safety_tests::{Chrome, Fixture};
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use std::{cell::Cell, rc::Rc};

    const AREA: &str = "New matrix";
    const REMEDY: &str = "Create the matrix or cancel the setup";

    fn runtime_form() -> Element {
        crate::objects::use_matrix_setup_preview_provider(false);
        let mut form_mounted = use_signal(|| true);
        let runtime = use_context::<Rc<crate::runtime::Runtime>>();
        let version = use_signal(|| 0u64);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        use_drop({
            let runtime = runtime.clone();
            move || runtime.unsubscribe()
        });
        let _ = version();
        let selected = use_signal(|| None);
        let anchor = use_signal(|| None);
        let workspace = use_signal(|| "Layout");
        let generation = use_signal(|| 1u64);
        let mount = super::super::matrix_setup_controller::use_matrix_setup(
            runtime, version, selected, anchor, workspace, generation,
        );
        let preview_mount = mount.clone();
        rsx! {
            button { aria_label: "Open matrix", onclick: move |_| mount.on_open.call(()), "Open matrix" }
            if form_mounted() {
                if let Some(projection) = mount.projection {
                    MatrixSetup { projection, on_create: mount.on_create, on_cancel: mount.on_cancel }
                }
            }
            // Keep the real controller projection/outlet alive when only the form unmounts:
            // otherwise projection retirement would mask a missing painter cleanup.
            svg { crate::objects::MatrixSetupCanvasPreview {
                setup: preview_mount, center: boardstudio_core::model::Vec2::default(),
                available_size: boardstudio_core::model::Vec2 { x: 200.0, y: 200.0 },
            } }
            button { onclick: move |_| form_mounted.set(false), "Unmount setup form" }
            button { onclick: move |_| form_mounted.set(true), "Remount setup form" }
            Chrome {}
        }
    }

    #[component]
    fn AdmissionProbe(
        owner: MatrixSetupOwner,
        version: Signal<u64>,
        status: Option<String>,
    ) -> Element {
        // Admission reads are non-reactive (`peek`/RefCell), so the runtime
        // version signal is what re-renders this probe on each submission;
        // the status prop changes on every preparation transition and keeps
        // the (otherwise memo-skipped) probe following the controller.
        let _ = version();
        let _ = &status;
        // Admission changes can land between rendered frames (a cached
        // preparation starts and submits within one), so the probe also
        // re-reads admission on its own ticker instead of trusting renders.
        let mut tick = use_signal(|| 0u64);
        let ticker_stop = use_hook(|| Rc::new(Cell::new(false)));
        use_drop({
            let ticker_stop = ticker_stop.clone();
            move || ticker_stop.set(true)
        });
        use_hook(move || {
            let ticker_stop = ticker_stop.clone();
            wasm_bindgen_futures::spawn_local(async move {
                loop {
                    gloo_timers::future::TimeoutFuture::new(10).await;
                    if ticker_stop.get() {
                        break;
                    }
                    tick += 1;
                }
            })
        });
        let _ = tick();
        let admission =
            try_use_context::<super::super::matrix_setup_controller::MatrixSetupAdmission>();
        let carries = |rows: u32, columns: u32| {
            let request = MatrixSetupCreateRequest {
                owner: owner.clone(),
                rows,
                columns,
                preset: MatrixSetupPreset::MxHotswap,
            };
            admission
                .as_ref()
                .is_some_and(|admission| admission.carries(&request))
        };
        let pending = admission
            .as_ref()
            .is_some_and(|admission| admission.is_pending());
        rsx! {
            output { id: "matrix-admission-probe",
                {format!("pending={pending} carries_first={} carries_second={}", carries(6, 6), carries(4, 4))}
            }
        }
    }

    fn runtime_direct_create_form() -> Element {
        crate::objects::use_matrix_setup_preview_provider(false);
        let runtime = use_context::<Rc<crate::runtime::Runtime>>();
        let version = use_signal(|| 0u64);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        use_drop({
            let runtime = runtime.clone();
            move || runtime.unsubscribe()
        });
        let _ = version();
        let selected = use_signal(|| None);
        let anchor = use_signal(|| None);
        let workspace = use_signal(|| "Layout");
        let generation = use_signal(|| 1u64);
        let mount = super::super::matrix_setup_controller::use_matrix_setup(
            runtime, version, selected, anchor, workspace, generation,
        );
        let first_mount = mount.clone();
        let second_mount = mount.clone();
        let mut direct_clicks = use_signal(|| 0u32);
        rsx! {
            button { aria_label: "Open matrix", onclick: move |_| mount.on_open.call(()), "Open matrix" }
            if let Some(projection) = mount.projection.clone() {
                AdmissionProbe { owner: projection.owner.clone(), version, status: projection.status.clone() }
                output { id: "direct-clicks", "{direct_clicks()}" }
                button {
                    onclick: move |_| {
                        direct_clicks += 1;
                        if let Some(current) = first_mount.projection.as_ref() {
                            first_mount.on_create.call(MatrixSetupCreateRequest {
                                owner: current.owner.clone(), rows: 6, columns: 6,
                                preset: MatrixSetupPreset::MxHotswap,
                            });
                        }
                    },
                    "Submit first create directly"
                }
                button {
                    onclick: move |_| {
                        direct_clicks += 1;
                        if let Some(current) = second_mount.projection.as_ref() {
                            second_mount.on_create.call(MatrixSetupCreateRequest {
                                owner: current.owner.clone(), rows: 4, columns: 4,
                                preset: MatrixSetupPreset::MxHotswap,
                            });
                        }
                    },
                    "Submit second create directly"
                }
                MatrixSetup { projection, on_create: mount.on_create, on_cancel: mount.on_cancel }
            }
            Chrome {}
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn matrix_create_on_create_twice_while_pending_admits_only_the_first_submission() {
        use boardstudio_core::model::{Board, ProjectDoc};
        use boardstudio_web_runtime::runtime::project_name_test_support as support;
        let f = Fixture::mount(runtime_direct_create_form, "Open matrix").await;
        let mut document = ProjectDoc::empty("creation-draft", "Creation draft");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            thickness: 1.6,
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            traces: vec![],
            vias: vec![],
        });
        support::replace_document(&f.runtime, "matrix-board", "board", document).await;
        f.click("Open matrix");
        wait_until("real Matrix Setup opens for direct create", || {
            f.root.query_selector("form").unwrap().is_some()
        })
        .await;
        let revision_before = f.runtime.model().accepted.unwrap().document.revision;
        let (mut entered, release) = support::gate_next_core_reply(&f.runtime);
        f.click("Submit first create directly");
        wait_until("first direct Create submits a real edit", || {
            support::drive_pending(&f.runtime);
            entered
                .try_recv()
                .expect("the first direct Create reaches Core")
                .is_some()
        })
        .await;
        // Bypass the rendered (disabled) Create button: invoke the controller
        // handler a second time for the same owner while the first is pending.
        f.click("Submit second create directly");
        // The guarded handler refuses synchronously, so this settle loop only
        // gives a would-be second preparation its chance to submit; it stops
        // early if a second submission ever replaces the first in admission.
        let probe_text = |f: &Fixture| {
            f.root
                .query_selector("#matrix-admission-probe")
                .unwrap()
                .expect("admission probe stays mounted while Create is pending")
                .text_content()
                .unwrap()
        };
        for _ in 0..300 {
            support::drive_pending(&f.runtime);
            gloo_timers::future::TimeoutFuture::new(10).await;
            if probe_text(&f).contains("carries_second=true") {
                break;
            }
        }
        assert_eq!(
            f.root
                .query_selector("#direct-clicks")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "2",
            "both direct on_create calls were delivered to the controller"
        );
        let alert_text = f
            .root
            .query_selector("[role='alert']")
            .unwrap()
            .map(|alert| alert.text_content().unwrap_or_default());
        assert!(
            alert_text.is_none(),
            "second on_create while pending must be refused silently by admission, got error: {alert_text:?}"
        );
        let probe = probe_text(&f);
        assert_eq!(
            probe,
            "pending=true carries_first=true carries_second=false",
            "second on_create while the first Create is pending must not replace the first submission; root text: {}",
            f.root.text_content().unwrap_or_default()
        );
        release.send(()).unwrap();
        support::run_pending(&f.runtime).await;
        wait_until("the first Create lands", || {
            support::drive_pending(&f.runtime);
            f.runtime
                .model()
                .accepted
                .is_some_and(|accepted| accepted.document.matrices.len() == 1)
        })
        .await;
        let accepted = f.runtime.model().accepted.unwrap();
        assert_eq!(
            (
                accepted.document.matrices[0].rows,
                accepted.document.matrices[0].columns
            ),
            (6, 6),
            "the landed matrix is the first submission's captured target"
        );
        assert_eq!(
            accepted.document.revision,
            revision_before + 1,
            "exactly one Create submission landed"
        );
        f.root.remove();
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn matrix_real_create_pending_exempts_only_submitted_draft() {
        use boardstudio_core::model::{Board, ProjectDoc};
        use boardstudio_web_runtime::runtime::project_name_test_support as support;
        use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog};
        let f = Fixture::mount(runtime_form, "Open matrix").await;
        let mut document = ProjectDoc::empty("creation-draft", "Creation draft");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            thickness: 1.6,
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            traces: vec![],
            vias: vec![],
        });
        support::replace_document(&f.runtime, "matrix-board", "board", document).await;
        f.click("Open matrix");
        wait_until("real Matrix Setup opens", || {
            f.root.query_selector("form").unwrap().is_some()
        })
        .await;
        key(&f, "Rows", "ArrowRight", "6").await;
        let ghost = "[aria-label='New matrix preview'] rect";
        wait_until("the real form paints 6 by 6 before submission", || {
            f.root.query_selector_all(ghost).unwrap().length() == 36
        })
        .await;
        f.click("Unmount setup form");
        wait_until(
            "form unmount clears paint even while the controller/outlet remain live",
            || {
                f.root.query_selector("form").unwrap().is_none()
                    && f.root.query_selector_all(ghost).unwrap().length() == 0
            },
        )
        .await;
        f.click("Remount setup form");
        wait_until("remount paints fresh defaults, not the old draft", || {
            f.root.query_selector_all(ghost).unwrap().length() == 30
        })
        .await;
        key(&f, "Rows", "ArrowRight", "6").await;
        f.deferred("Update Now (toast)", AREA, REMEDY).await;
        let (mut entered, release) =
            support::gate_next_core_reply_then_fail(&f.runtime, "held matrix failed");
        f.click("Create matrix");
        // Preparation is a production async owner, not a synthetic host admission.
        wait_until("matrix preparation submits a real edit", || {
            support::drive_pending(&f.runtime);
            entered
                .try_recv()
                .expect("the real matrix Create reaches Core")
                .is_some()
        })
        .await;
        assert_eq!(
            f.root.query_selector_all(ghost).unwrap().length(),
            0,
            "Create clears the ghost while the real edit is pending"
        );
        f.click("Update Now (header)");
        let Some(UpdateDialog::Deferred { blockers }) = f.runtime.update_dialog() else {
            panic!("held Core work must independently block update");
        };
        let unresolved = blockers.iter().any(|blocker| {
            matches!(blocker,
            UpdateBlocker::UnresolvedDraft { area, .. } if area == AREA)
        });
        f.runtime.cancel_update_dialog();
        release.send(()).unwrap();
        support::run_pending(&f.runtime).await;
        wait_until("matrix Create failure preserves the mounted form", || {
            f.root
                .text_content()
                .unwrap()
                .contains("held matrix failed")
        })
        .await;
        f.click("Update Now (toast)");
        assert!(
            matches!(f.runtime.update_dialog(), Some(UpdateDialog::Deferred { blockers })
            if blockers.contains(&UpdateBlocker::UnresolvedDraft { area: AREA.into(), remedy: REMEDY.into() })),
            "failure never clears the matrix draft"
        );
        f.runtime.cancel_update_dialog();
        f.click("Cancel");
        wait_until("real matrix owner cancels", || {
            f.root.query_selector("form").unwrap().is_none()
                && f.root.query_selector_all(ghost).unwrap().length() == 0
        })
        .await;
        f.root.remove();
        assert!(
            !unresolved,
            "same-key in-flight matrix Create carries the live draft"
        );
    }

    // Reuse the creation-form Update Now fixture; only the owner's controls differ.
    fn form() -> Element {
        let accepted = use_context::<Rc<crate::runtime::Runtime>>()
            .model()
            .accepted
            .unwrap();
        let requests = use_context::<Rc<Cell<u32>>>();
        let mut mounted = use_signal(|| true);
        let mut hidden = use_signal(|| false);
        let mut resolve = use_signal(|| false);
        let mut refused = use_signal(|| false);
        rsx! {
            div { hidden: hidden(),
                if mounted() {
                    MatrixSetup {
                        projection: MatrixSetupProjection {
                            owner: MatrixSetupOwner {
                                editor_instance_id: 1, open_id: 1, scope_generation: 1,
                                scope: Scope { session_epoch: accepted.session_epoch, document_id: accepted.document.id.clone(), board_id: "board".into(), instance_id: None },
                                board_id: "board".into(), snapshot_token: SnapshotToken(1), revision: 0,
                            },
                            editable: true, can_cancel: true,
                            error: refused().then(|| "Creation refused".into()), status: None,
                        },
                        on_cancel: move |_| mounted.set(false),
                        on_create: move |request: MatrixSetupCreateRequest| {
                            assert_eq!((request.rows, request.columns), (5, 6));
                            assert_eq!(request.preset, MatrixSetupPreset::MxHotswap);
                            requests.set(requests.get() + 1);
                            if resolve() { mounted.set(false); } else { refused.set(true); }
                        },
                    }
                }
            }
            Chrome {}
            button { onclick: move |_| hidden.set(true), "Hide form" }
            button { onclick: move |_| mounted.set(false), "Unmount form" }
            button { onclick: move |_| { hidden.set(false); mounted.set(true); refused.set(false); }, "Mount form" }
            button { onclick: move |_| resolve.set(true), "Allow creation" }
            if resolve() { output { "Creation allowed" } }
        }
    }

    fn slider(root: &web_sys::Element, label: &str) -> web_sys::Element {
        root.query_selector(&format!("[role='slider'][aria-label='{label}']"))
            .unwrap()
            .unwrap()
    }

    async fn key(f: &Fixture, label: &str, key: &str, expected: &str) {
        let target = slider(&f.root, label);
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(key);
        init.set_bubbles(true);
        target
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        wait_until(
            "matrix slider changes through its production writer",
            || {
                slider(&f.root, label)
                    .get_attribute("aria-valuenow")
                    .as_deref()
                    == Some(expected)
            },
        )
        .await;
    }

    async fn pick(f: &Fixture, label: &str) {
        f.click(label);
        wait_until("matrix preset choice renders", || {
            boardstudio_web_ui_shared::test_role::by_role_name(&f.root, "button", label)
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref()
                == Some("true")
        })
        .await;
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn matrix_mount_slider_revert_hidden_and_unmount_gate() {
        let f = Fixture::mount(form, AREA).await;
        assert_eq!(
            slider(&f.root, "Rows")
                .get_attribute("aria-valuenow")
                .as_deref(),
            Some("5")
        );
        assert_eq!(
            slider(&f.root, "Columns")
                .get_attribute("aria-valuenow")
                .as_deref(),
            Some("6")
        );
        f.clear("Update Now (toast)").await;
        f.clear("Update Now (header)").await;
        for (label, changed, original) in [("Rows", "6", "5"), ("Columns", "7", "6")] {
            key(&f, label, "ArrowRight", changed).await;
            f.no_invalid_marker();
            for opener in ["Update Now (toast)", "Update Now (header)"] {
                f.deferred(opener, AREA, REMEDY).await;
                assert_eq!(
                    slider(&f.root, label)
                        .get_attribute("aria-valuenow")
                        .as_deref(),
                    Some(changed)
                );
            }
            key(&f, label, "ArrowLeft", original).await;
            f.clear("Update Now (header)").await;
        }
        key(&f, "Rows", "ArrowRight", "6").await;
        f.click("Hide form");
        wait_until("matrix is hidden but mounted", || {
            f.root.query_selector("[hidden] form").unwrap().is_some()
        })
        .await;
        f.deferred("Update Now (header)", AREA, REMEDY).await;
        f.root.remove();
    }

    // Removal is already safe without an adapter: record this as a proof gap, not RED.
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn matrix_own_controls_and_unsubmitted_unmount_retire_gate() {
        let f = Fixture::mount(form, AREA).await;
        f.clear("Update Now (toast)").await;
        key(&f, "Rows", "ArrowRight", "6").await;
        f.click("Unmount form");
        wait_until("unsubmitted matrix retires", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        f.clear("Update Now (header)").await;
        f.click("Mount form");
        wait_until("matrix reopens at defaults", || {
            f.root.query_selector("form").unwrap().is_some()
        })
        .await;
        assert_eq!(
            slider(&f.root, "Rows")
                .get_attribute("aria-valuenow")
                .as_deref(),
            Some("5")
        );
        pick(&f, "Hotswap").await;
        f.click("Cancel");
        wait_until("own Cancel retires matrix", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        f.clear("Update Now (toast)").await;
        f.click("Mount form");
        wait_until("matrix reopens for creation", || {
            f.root.query_selector("form").unwrap().is_some()
        })
        .await;
        pick(&f, "Hotswap").await;
        f.click("Allow creation");
        wait_until("fixture allows creation", || {
            f.root.text_content().unwrap().contains("Creation allowed")
        })
        .await;
        f.click("Create matrix");
        wait_until("own creation resolves matrix", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        assert_eq!(f.requests.get(), 1);
        f.clear("Update Now (header)").await;
        f.root.remove();
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn matrix_preset_revert_refusal_cancel_and_creation_gate() {
        let f = Fixture::mount(form, AREA).await;
        f.clear("Update Now (toast)").await;
        for (changed, original) in [
            ("ChocLow profile", "MXFull height"),
            ("Hotswap", "Solder"),
            ("Per-key RGB", "None"),
        ] {
            pick(&f, changed).await;
            f.no_invalid_marker();
            f.deferred("Update Now (toast)", AREA, REMEDY).await;
            pick(&f, original).await;
            f.clear("Update Now (header)").await;
        }
        pick(&f, "Hotswap").await;
        f.click("Create matrix");
        wait_until("creation refusal arrives", || {
            f.root.text_content().unwrap().contains("Creation refused")
        })
        .await;
        assert_eq!(f.requests.get(), 1);
        f.deferred("Update Now (header)", AREA, REMEDY).await;
        f.root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "matrix_setup_focus_tests.rs"]
mod focus_tests;
