//! Private, source-parity projection for the new mirrored-pair placement.
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
    parse_matrix_dimensions,
};
use crate::matrix_setup_operation::MatrixSetupPreset;
use crate::mirrored_pair_geometry::MirroredPairProjection;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Vec2;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::field::{Field, FieldLabel};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupInput};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::collections::HashSet;

mod canvas_overlay;
pub use canvas_overlay::MirroredPairCanvasOverlay;

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairRequest {
    pub owner: MirroredPairOwner,
    pub left_name: String,
    pub right_name: String,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
    pub gap_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairFormProjection {
    pub owner: MirroredPairOwner,
    pub values: MirroredPairFormValues,
    pub editable: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairFormValues {
    pub left_name: String,
    pub right_name: String,
    pub rows: String,
    pub columns: String,
    pub preset: MatrixSetupPreset,
    pub gap_mm: String,
}

impl Default for MirroredPairFormValues {
    fn default() -> Self {
        Self {
            left_name: "Left half".into(),
            right_name: "Right half".into(),
            rows: "3".into(),
            columns: "5".into(),
            preset: MatrixSetupPreset::MxSolder,
            gap_mm: "40".into(),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct MirroredPairMount {
    pub form: Option<MirroredPairFormProjection>,
    pub placement: Option<MirroredPairPlacement>,
    pub can_open: bool,
    /// Owns the placement interaction from form admission through pending save settlement.
    pub owns_canvas: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
    pub on_move: EventHandler<MirroredPairMove>,
    pub on_commit: EventHandler<MirroredPairMove>,
    pub on_created: EventHandler<MirroredPairCreated>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairPlacement {
    pub owner: MirroredPairOwner,
    pub pair: MirroredPairProjection,
    pub left_scene: boardstudio_core::model::MatrixScene,
    pub right_scene: boardstudio_core::model::MatrixScene,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairMove {
    pub owner: MirroredPairOwner,
    pub center: Vec2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairCreated {
    pub owner: MirroredPairOwner,
    pub result_token: SnapshotToken,
    pub result_revision: u64,
    pub scope: Scope,
    pub left_layout_id: String,
    pub right_layout_id: String,
    pub left_matrix_id: String,
    pub right_matrix_id: String,
}

#[derive(Props, Clone, PartialEq)]
pub struct MirroredPairFormProps {
    pub projection: MirroredPairFormProjection,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
}

#[component]
pub fn MirroredPairForm(props: MirroredPairFormProps) -> Element {
    let values = props.projection.values.clone();
    let baseline = try_use_context::<super::mirrored_pair_controller::MirroredPairDraftBaseline>();
    let starting_values = use_hook(|| {
        baseline.map_or_else(
            || values.clone(),
            |baseline| baseline.starting_values(&props.projection.owner, &values),
        )
    });
    let mut left_name = use_signal(|| values.left_name);
    let mut right_name = use_signal(|| values.right_name);
    let rows = use_signal(|| values.rows);
    let columns = use_signal(|| values.columns);
    let mut gap = use_signal(|| values.gap_mm);
    let (choc_default, hotswap_default, rgb_default) = preset_choices(values.preset);
    let family = use_signal(|| Some(HashSet::from([usize::from(choc_default)])));
    let mounting = use_signal(|| Some(HashSet::from([usize::from(hotswap_default)])));
    let lighting = use_signal(|| Some(HashSet::from([usize::from(rgb_default)])));
    let row_slider = use_signal(|| {
        Some(
            rows()
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| (1.0..=12.0).contains(value))
                .unwrap_or(3.0),
        )
    });
    let column_slider = use_signal(|| {
        Some(
            columns()
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| (1.0..=18.0).contains(value))
                .unwrap_or(5.0),
        )
    });
    let runtime = try_use_context::<std::rc::Rc<crate::runtime::Runtime>>();
    let guard = use_signal(|| None::<boardstudio_web_runtime::draft_owners::DraftOwnerGuard>);
    let project_scope = (
        props.projection.owner.scope.document_id.clone(),
        props.projection.owner.scope.session_epoch,
    );
    use_effect(use_reactive((&project_scope,), {
        let runtime = runtime.clone();
        let mut guard = guard;
        move |(project_scope,)| {
            let Some(runtime) = runtime.as_ref() else {
                return;
            };
            if let Some(old) = guard.write().take() {
                runtime.retire_draft_owner(old);
            }
            let baseline = starting_values.clone();
            guard.set(Some(runtime.register_draft_owner(
                boardstudio_web_runtime::draft_owners::DraftOwnerRegistration {
                    project_scope,
                    workspace: "Layout",
                    owner: "mirrored-pair-setup",
                    admission_key: "create",
                    area: "New mirrored pair".into(),
                    remedy: "Preview the pair or cancel the setup".into(),
                    is_unresolved: std::rc::Rc::new(move || {
                        *left_name.peek() != baseline.left_name
                            || *right_name.peek() != baseline.right_name
                            || *rows.peek() != baseline.rows
                            || *columns.peek() != baseline.columns
                            || *gap.peek() != baseline.gap_mm
                            || matrix_setup_preset(
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
                            ) != baseline.preset
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
    let left_name_value = left_name();
    let right_name_value = right_name();
    let rows_value = rows();
    let columns_value = columns();
    let gap_value = gap();
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = matrix_setup_preset(choc, hotswap, rgb);
    let dimensions = parse_matrix_dimensions(&rows_value, &columns_value);
    let gap_mm = gap_value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0);
    let valid_names = !left_name_value.trim().is_empty()
        && !right_name_value.trim().is_empty()
        && left_name_value.trim() != right_name_value.trim();
    let can_preview =
        props.projection.editable && dimensions.is_some() && gap_mm.is_some() && valid_names;
    let owner = props.projection.owner.clone();
    let on_preview = props.on_preview;
    let left_submit = left_name_value.clone();
    let right_submit = right_name_value.clone();
    let preset_submit = selected_preset;
    let gap_submit = gap_value.clone();
    let owner_submit = owner.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        let Some((rows, columns)) = dimensions else {
            return;
        };
        let Some(gap_mm) = gap_submit
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
        else {
            return;
        };
        let left_name = left_submit.trim().to_owned();
        let right_name = right_submit.trim().to_owned();
        if left_name.is_empty() || right_name.is_empty() || left_name == right_name {
            return;
        }
        on_preview.call(MirroredPairRequest {
            owner: owner_submit.clone(),
            left_name,
            right_name,
            rows,
            columns,
            preset: preset_submit,
            gap_mm,
        });
    };
    let error = props.projection.error.clone();
    let status = props.projection.status.clone();
    let on_cancel = props.on_cancel;

    let left_id = format!("layout-input-{}-left", dioxus::core::current_scope_id().0);
    let right_id = format!("layout-input-{}-right", dioxus::core::current_scope_id().0);
    let gap_id = format!("layout-input-{}-gap", dioxus::core::current_scope_id().0);
    rsx! {
        section {
            class: "pointer-events-auto relative box-border flex max-h-full w-[min(380px,100%)] flex-col gap-0 overflow-y-auto rounded-md border border-border-strong bg-card p-6 text-foreground shadow-md max-[760px]:w-[min(380px,calc(100%-8px))] max-[760px]:p-[18px]",
            aria_label: "New mirrored pair",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                }
            },
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-xl!", h2 { "Mirrored pair" } }
            p { class: "mt-2.5 mb-5 text-[13px] leading-normal text-muted-foreground",
                "Linked key assemblies, diode settings and components. Substitute a component on either half when needed."
            }
            form { onsubmit: submit, class: "grid gap-2",
                div { class: "grid grid-cols-2 gap-x-4 gap-y-3", role: "group", aria_label: "Mirrored pair dimensions",
                    Field {
                        FieldLabel { html_for: left_id.clone(), "Left layout" }
                        InputGroup { class: "max-[760px]:h-control-touch",
                            InputGroupInput {
                                class: "max-[760px]:h-control-touch", aria_label: "Left layout name", id: left_id,
                                required: true, value: "{left_name_value}", disabled: !props.projection.editable,
                                oninput: move |event: FormEvent| left_name.set(event.value()),
                                attributes: vec![dioxus::events::onmounted(move |event: MountedEvent| async move { let _ = event.set_focus(true).await; })],
                            }
                        }
                    }
                    Field {
                        FieldLabel { html_for: right_id.clone(), "Right layout" }
                        InputGroup { class: "max-[760px]:h-control-touch",
                            InputGroupInput {
                                class: "max-[760px]:h-control-touch", aria_label: "Right layout name", id: right_id,
                                required: true, value: "{right_name_value}", disabled: !props.projection.editable,
                                oninput: move |event: FormEvent| right_name.set(event.value()),
                            }
                        }
                    }
                }
                MatrixDimensionSliders {
                    rows, columns, row_slider, column_slider,
                    rows_label: "Rows per half", columns_label: "Columns per half", editable: props.projection.editable,
                }
                MatrixAssemblyPicker { family, mounting, lighting, editable: props.projection.editable }
                MatrixPreview { rows: rows_value.clone(), columns: columns_value.clone() }
                Field {
                    FieldLabel { html_for: gap_id.clone(), "Gap between key edges (mm)" }
                    InputGroup { class: "max-[760px]:h-control-touch",
                        InputGroupInput {
                            class: "max-[760px]:h-control-touch", aria_label: "Gap between key edges (mm)", id: gap_id,
                            r#type: InputType::Number, min: "0", step: "any", required: true,
                            value: "{gap_value}", disabled: !props.projection.editable,
                            oninput: move |event: FormEvent| gap.set(event.value()),
                        }
                    }
                }
                p { class: "mt-2.5 mb-5 border-t border-border pt-3.5 text-[13px] leading-normal text-muted-foreground",
                    "Edit either half to update both. Unlink in the inspector for independent geometry."
                }
                if let Some(error) = error { Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } } }
                if let Some(status) = status { div { role: "status", Callout { variant: CalloutVariant::Info, "{status}" } } }
                footer { class: "flex justify-end gap-2",
                    Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| on_cancel.call(owner.clone()), "Cancel" }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !can_preview, "Preview placement" }
                }
            }
        }
    }
}

fn preset_choices(preset: MatrixSetupPreset) -> (bool, bool, bool) {
    match preset {
        MatrixSetupPreset::MxSolder => (false, false, false),
        MatrixSetupPreset::MxHotswap => (false, true, false),
        MatrixSetupPreset::ChocSolder => (true, false, false),
        MatrixSetupPreset::ChocHotswap => (true, true, false),
        MatrixSetupPreset::MxRgb => (false, false, true),
        MatrixSetupPreset::ChocRgb => (true, false, true),
        MatrixSetupPreset::MxHotswapRgb => (false, true, true),
        MatrixSetupPreset::ChocHotswapRgb => (true, true, true),
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
pub(super) mod draft_safety_tests {
    use super::*;
    use boardstudio_web_host::host::{UpdateSignals, UpdateWatch};
    use boardstudio_web_host::offline::{AdoptError, UpdateSnapshot};
    use boardstudio_web_runtime::runtime::project_name_test_support as support;
    use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog, UpdateOpener};
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::JsCast;

    // Same mounted begin_update seam as ticket 011; chrome is fixture-only.
    // Shared with existing-half to avoid a second runtime/update driver.
    pub(in crate::objects) struct Fixture {
        pub runtime: Rc<crate::runtime::Runtime>,
        pub root: web_sys::Element,
        signals: Rc<WaitingUpdate>,
        pub requests: Rc<Cell<u32>>,
    }

    struct WaitingUpdate(Cell<u32>);
    impl UpdateSignals for WaitingUpdate {
        fn snapshot(&self) -> UpdateSnapshot {
            UpdateSnapshot {
                waiting: true,
                installing: false,
                controlled: true,
            }
        }
        fn watch(&self, _: Rc<dyn Fn()>) -> Box<dyn UpdateWatch> {
            struct Stop;
            impl UpdateWatch for Stop {
                fn stop(&mut self) {}
            }
            Box::new(Stop)
        }
        fn adopt_waiting(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AdoptError>>>> {
            self.0.set(self.0.get() + 1);
            // The worker refusal rearms the gate for the next assertion without reloading.
            Box::pin(async { Err(AdoptError::OtherTabs) })
        }
        fn reload(&self) {}
    }

    #[component]
    pub(in crate::objects) fn Chrome() -> Element {
        let runtime = use_context::<Rc<crate::runtime::Runtime>>();
        let header = runtime.clone();
        rsx! {
            button { onclick: move |_| runtime.begin_update(UpdateOpener::Toast), "Update Now (toast)" }
            button { onclick: move |_| header.begin_update(UpdateOpener::Header), "Update Now (header)" }
        }
    }

    impl Fixture {
        pub async fn mount(make: fn() -> Element, label: &str) -> Self {
            let runtime = support::new_runtime();
            support::open_document(
                &runtime,
                boardstudio_core::model::ProjectDoc::empty("creation-draft", "Creation draft"),
            )
            .await;
            let signals = Rc::new(WaitingUpdate(Cell::new(0)));
            runtime.observe_update_waiting(signals.clone());
            let document = web_sys::window().unwrap().document().unwrap();
            let root = document.create_element("div").unwrap();
            document.body().unwrap().append_child(&root).unwrap();
            let dom = VirtualDom::new(make);
            dom.provide_root_context(runtime.clone());
            let requests = Rc::new(Cell::new(0));
            dom.provide_root_context(requests.clone());
            dioxus_web::launch::launch_virtual_dom(
                dom,
                dioxus_web::Config::new().rootnode(root.clone().into()),
            );
            wait_until("creation form mounts", || {
                root.query_selector(&format!("[aria-label='{label}']"))
                    .unwrap()
                    .is_some()
            })
            .await;
            Self {
                runtime,
                root,
                signals,
                requests,
            }
        }
        pub fn click(&self, label: &str) {
            by_role_name(&self.root, "button", label).unwrap().click();
        }
        pub async fn input(&self, label: &str, value: &str) {
            let field = self
                .root
                .query_selector(&format!("input[aria-label='{label}']"))
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            field.set_value(value);
            let init = web_sys::EventInit::new();
            init.set_bubbles(true);
            field
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
                .unwrap();
            support::run_pending(&self.runtime).await;
        }
        pub fn value(&self, label: &str) -> String {
            self.root
                .query_selector(&format!("input[aria-label='{label}']"))
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap()
                .value()
        }
        pub async fn clear(&self, opener: &str) {
            let before = self.signals.0.get();
            self.click(opener);
            wait_until("clean form reaches the worker", || {
                self.signals.0.get() > before
                    && matches!(self.runtime.update_dialog(), Some(UpdateDialog::OtherTabs))
            })
            .await;
            self.runtime.cancel_update_dialog();
        }
        pub async fn deferred(&self, opener: &str, area: &str, remedy: &str) {
            let before = self.signals.0.get();
            self.click(opener);
            support::run_pending(&self.runtime).await;
            assert_eq!(
                self.runtime.update_dialog(),
                Some(UpdateDialog::Deferred {
                    blockers: vec![UpdateBlocker::UnresolvedDraft {
                        area: area.into(),
                        remedy: remedy.into()
                    }],
                }),
                "the actual Update Now gate must name this creation draft"
            );
            assert_eq!(
                self.signals.0.get(),
                before,
                "no adoption while the draft is unresolved"
            );
            self.runtime.cancel_update_dialog();
        }
        pub fn submit(&self) {
            let init = web_sys::EventInit::new();
            init.set_bubbles(true);
            init.set_cancelable(true);
            self.root
                .query_selector("form")
                .unwrap()
                .unwrap()
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("submit", &init).unwrap())
                .unwrap();
        }
        pub fn no_invalid_marker(&self) {
            let invalid = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("[aria-invalid='true']")
                .unwrap();
            assert!(
                invalid.is_none(),
                "P2 requires no invalid field anywhere in the document: {:?}",
                invalid.map(|field| field.outer_html())
            );
        }
    }

    fn form() -> Element {
        let accepted = use_context::<Rc<crate::runtime::Runtime>>()
            .model()
            .accepted
            .unwrap();
        let mut mounted = use_signal(|| true);
        let mut hidden = use_signal(|| false);
        let mut refused = use_signal(|| false);
        let requests = use_context::<Rc<Cell<u32>>>();
        rsx! {
            div { hidden: hidden(),
                if mounted() {
                    MirroredPairForm {
                        projection: MirroredPairFormProjection {
                            owner: MirroredPairOwner {
                                editor_instance_id: 1, open_id: 1, scope_generation: 1,
                                scope: Scope { session_epoch: accepted.session_epoch, document_id: accepted.document.id.clone(), board_id: "board".into(), instance_id: None },
                                board_id: "board".into(), snapshot_token: SnapshotToken(1), revision: 0,
                            },
                            values: MirroredPairFormValues { left_name: "Mounted left".into(), ..Default::default() },
                            editable: true, error: refused().then(|| "Preview refused".into()), status: None,
                        },
                        on_cancel: move |_| mounted.set(false),
                        on_preview: move |_| { requests.set(requests.get() + 1); refused.set(true); },
                    }
                }
            }
            Chrome {}
            button { onclick: move |_| hidden.set(true), "Hide form" }
            button { onclick: move |_| mounted.set(false), "Unmount form" }
            button { onclick: move |_| mounted.set(true), "Mount form" }
        }
    }

    const AREA: &str = "New mirrored pair";
    const REMEDY: &str = "Preview the pair or cancel the setup";

    fn runtime_form() -> Element {
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
        let workspace = use_signal(|| "Layout");
        let generation = use_signal(|| 1u64);
        let canvas =
            use_hook(crate::presentation::canvas_interaction::CanvasInteractionArbiter::default);
        let created = use_callback(|_: MirroredPairCreated| {});
        let mount = super::super::mirrored_pair_controller::use_mirrored_pair(
            runtime, version, workspace, generation, created, canvas,
        );
        rsx! {
            button { aria_label: "Open pair", onclick: move |_| mount.on_open.call(()), "Open pair" }
            if let Some(projection) = mount.form {
                MirroredPairForm { projection, on_preview: mount.on_preview, on_cancel: mount.on_cancel }
            }
            if let Some(placement) = mount.placement {
                button { onclick: move |_| mount.on_commit.call(MirroredPairMove { owner: placement.owner.clone(), center: Vec2::default() }), "Commit pair" }
            }
            Chrome {}
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn mirrored_pair_real_preview_and_create_failure_preserve_draft() {
        use boardstudio_core::model::{Board, ProjectDoc};
        use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog};
        let f = Fixture::mount(runtime_form, "Open pair").await;
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
        support::replace_document(&f.runtime, "pair-board", "board", document).await;
        f.click("Open pair");
        wait_until("real pair setup opens", || {
            f.root.query_selector("form").unwrap().is_some()
        })
        .await;
        // Drive an observable disabled/enabled cycle so Preview's closure carries this value.
        f.input("Left layout name", "Right half").await;
        wait_until("duplicate name renders disabled Preview", || {
            f.root
                .query_selector("button[type=submit]")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        })
        .await;
        f.no_invalid_marker();
        f.submit();
        support::run_pending(&f.runtime).await;
        f.deferred("Update Now (header)", AREA, REMEDY).await;
        f.input("Left layout name", "Changed left").await;
        wait_until("changed name renders enabled Preview", || {
            !f.root
                .query_selector("button[type=submit]")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        })
        .await;
        f.click("Preview placement");
        // Still mounted in Preparing: Preview is not an admitted Create, even for the same value.
        assert!(f.root.query_selector("form").unwrap().is_some());
        f.deferred("Update Now (toast)", AREA, REMEDY).await;
        wait_until(
            "real pair reaches placement or reports its preparation refusal",
            || {
                f.root.text_content().unwrap().contains("Commit pair")
                    || f.root.query_selector("[role=alert]").unwrap().is_some()
            },
        )
        .await;
        assert!(
            f.root.text_content().unwrap().contains("Commit pair"),
            "cannot test real pair Create without its production preview: {}",
            f.root.text_content().unwrap()
        );
        assert!(
            f.root.query_selector("form").unwrap().is_none(),
            "Placement genuinely unmounts setup"
        );
        let (entered, release) =
            support::gate_next_core_reply_then_fail(&f.runtime, "held pair failed");
        f.click("Commit pair");
        support::drive_pending(&f.runtime);
        entered.await.expect("real pair Create reaches Core");
        let form_unmounted = f.root.query_selector("form").unwrap().is_none();
        f.click("Update Now (header)");
        let pending_without_draft = matches!(f.runtime.update_dialog(), Some(UpdateDialog::Deferred { blockers })
            if !blockers.iter().any(|blocker| matches!(blocker, UpdateBlocker::UnresolvedDraft { area, .. } if area == AREA)));
        f.runtime.cancel_update_dialog();
        release.send(()).unwrap();
        support::run_pending(&f.runtime).await;
        wait_until("real pair Create failure remounts setup", || {
            f.root.query_selector("form").unwrap().is_some()
                && f.root.text_content().unwrap().contains("held pair failed")
        })
        .await;
        f.click("Update Now (header)");
        assert!(
            matches!(f.runtime.update_dialog(), Some(UpdateDialog::Deferred { blockers })
            if blockers.contains(&UpdateBlocker::UnresolvedDraft { area: AREA.into(), remedy: REMEDY.into() })),
            "failure must not turn the retained dirty pair values into a new clean baseline"
        );
        f.runtime.cancel_update_dialog();
        f.click("Cancel");
        wait_until("pair cancels", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        f.root.remove();
        assert!(
            form_unmounted,
            "Create stays in Placement, not a mounted Pending form"
        );
        assert!(
            pending_without_draft,
            "held Create independently blocks update, but the unmounted pair form does not"
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn mirrored_pair_mount_revert_failure_hidden_and_unmount_gate() {
        let f = Fixture::mount(form, "Left layout name").await;
        f.clear("Update Now (toast)").await;
        f.input("Left layout name", "Different left").await;
        for opener in ["Update Now (toast)", "Update Now (header)"] {
            f.deferred(opener, AREA, REMEDY).await;
            assert_eq!(f.value("Left layout name"), "Different left");
        }
        f.input("Left layout name", "Mounted left").await;
        f.clear("Update Now (header)").await;
        f.input("Left layout name", "Different left").await;
        f.click("Preview placement");
        wait_until("preview refusal arrives", || {
            f.root.text_content().unwrap().contains("Preview refused")
        })
        .await;
        f.deferred("Update Now (toast)", AREA, REMEDY).await;
        f.click("Hide form");
        wait_until("pair is hidden but mounted", || {
            f.root.query_selector("[hidden] form").unwrap().is_some()
        })
        .await;
        f.deferred("Update Now (header)", AREA, REMEDY).await;
        f.click("Unmount form");
        wait_until("unsubmitted pair unmounts", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        f.clear("Update Now (toast)").await;
        f.root.remove();
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn mirrored_pair_silently_rejected_names_and_cancel_gate() {
        let f = Fixture::mount(form, "Left layout name").await;
        // Nonempty duplicate names pass native required validation, but submit returns silently.
        f.input("Left layout name", "Right half").await;
        f.no_invalid_marker();
        f.deferred("Update Now (toast)", AREA, REMEDY).await;
        f.click("Unmount form");
        wait_until("pair unmounts without a submit attempt", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        f.clear("Update Now (header)").await;
        f.click("Mount form");
        wait_until("pair remounts", || {
            f.root.query_selector("form").unwrap().is_some()
        })
        .await;
        f.clear("Update Now (toast)").await;
        f.input("Left layout name", "Right half").await;
        wait_until("duplicate names disable preview after render", || {
            f.root
                .query_selector("button[type=submit]")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        })
        .await;
        f.submit();
        support::run_pending(&f.runtime).await;
        assert_eq!(
            f.requests.get(),
            0,
            "invalid submit returns without a request"
        );
        f.no_invalid_marker();
        f.deferred("Update Now (header)", AREA, REMEDY).await;
        assert_eq!(f.value("Left layout name"), "Right half");
        f.click("Cancel");
        wait_until("cancel retires pair form", || {
            f.root.query_selector("form").unwrap().is_none()
        })
        .await;
        f.clear("Update Now (header)").await;
        f.root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "mirrored_pair/canvas_overlay_tests.rs"]
mod canvas_overlay_tests;
