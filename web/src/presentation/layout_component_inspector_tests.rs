use super::inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorProjection, MirroredHalfProjection,
};
use super::*;
use boardstudio_application::{AcceptedSnapshot, ReadModel, Scope, SessionEpoch};
use boardstudio_core::model::{
    Board, Constraint, ConstraintEndpoint, ConstraintRelation, Layout, LayoutMirrorLink, Matrix,
    MatrixCell, MatrixScene, MatrixSceneCell, MirroredCorrespondence, MirroredItemKind,
    PartDefinition, PartKind, PartOutline, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
};
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// The document the fixture and the mounted runtime share: a board, two positioned
/// parts and the two mirrored layouts the projections read.
fn fixture_document(configure: impl FnOnce(&mut ProjectDoc)) -> ProjectDoc {
    let mut document = ProjectDoc::empty("inspector-doc", "Inspector fixture");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["selected-part".into(), "source-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: "component-definition".into(),
        name: "Fixture controller".into(),
        kind: PartKind::Controller,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: Some("Imported courtyard is approximate.".into()),
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    });
    for (id, reference, x) in [("selected-part", "U1", 4.0), ("source-part", "U2", 10.0)] {
        document.parts.push(Part {
            keycap: None,
            outline: (id == "selected-part").then_some(PartOutline {
                excluded: false,
                margin: Some(2.5),
                allow_body_overhang: true,
            }),
            id: id.into(),
            definition_id: "component-definition".into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 3.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            anchor: None,
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
    }
    document.layouts.push(Layout {
        id: "source-layout".into(),
        name: "Left half".into(),
        board_id: "board".into(),
        matrix_ids: vec!["matrix".into()],
        part_ids: vec!["selected-part".into()],
        mirror_link: None,
    });
    document.layouts.push(Layout {
        id: "paired-layout".into(),
        name: "Right half".into(),
        board_id: "board".into(),
        matrix_ids: vec!["paired-matrix".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            correspondences: vec![MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "matrix".into(),
                mirror_id: "paired-matrix".into(),
            }],
            exceptions: vec![],
            source_id: "source-layout".into(),
            axis_x: 0.0,
            split_gap: None,
            spacing: None,
        }),
    });
    configure(&mut document);
    document
}

fn fixture() -> (ReadModel, objects::ScopedTreeContext) {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "inspector-doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let document = fixture_document(|document| {
        document.revision = 9;
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "selected-part")
            .expect("the fixture selects a part")
            .locked = Some(true);
    });
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(13),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 9,
            transaction_id: "accepted-fixture".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        }),
    };
    let mut model = ReadModel {
        accepted: Some(snapshot),
        active_board_id: "board".into(),
        selected_part_ids: vec!["selected-part".into()],
        ..ReadModel::default()
    };
    let context = objects::context_for_part(&model, "selected-part")
        .expect("fixture component has a current standalone tree context");
    let selected = objects::ScopedTreeContext { scope, context };
    model.selection_anchor_id = Some("selected-part".into());
    (model, selected)
}

#[derive(Clone)]
struct MountedProbe {
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    selected_part: std::rc::Rc<RefCell<Option<String>>>,
    context_override: std::rc::Rc<RefCell<Option<objects::ScopedTreeContext>>>,
    adapter_context: std::rc::Rc<RefCell<Option<objects::ScopedTreeContext>>>,
    selection_kind: std::rc::Rc<RefCell<Option<Signal<objects::LayoutSelectionKind>>>>,
    projection: std::rc::Rc<RefCell<Option<LayoutComponentInspectorProjection>>>,
    action: std::rc::Rc<RefCell<Option<EventHandler<LayoutComponentInspectorAction>>>>,
    pending_edits:
        std::rc::Rc<RefCell<Option<Signal<layout_component_edits::LayoutComponentInspectorEdits>>>>,
    render_generation: std::rc::Rc<RefCell<Option<Signal<u64>>>>,
    mirrored_half_override: std::rc::Rc<RefCell<Option<MirroredHalfProjection>>>,
    last_action: std::rc::Rc<RefCell<Option<LayoutComponentInspectorAction>>>,
    root_id: &'static str,
}

impl MountedProbe {
    fn select_parts(&self, part_ids: Vec<String>) {
        self.runtime
            .submit(boardstudio_application::Event::SelectParts {
                operation_id: self.runtime.operation(),
                part_ids,
                range_part_ids: Vec::new(),
                mode: boardstudio_application::SelectionMode::Replace,
            });
        boardstudio_web_test_runtime::project_name_test_support::drive_pending(&self.runtime);
    }

    fn select(&self, part_id: Option<&str>) {
        *self.selected_part.borrow_mut() = part_id.map(str::to_owned);
        self.context_override.borrow_mut().take();
        self.select_parts(part_id.into_iter().map(str::to_owned).collect());
    }

    fn select_component_from_finding(&self, part_id: &str) {
        *self.selected_part.borrow_mut() = Some(part_id.to_owned());
        self.select_parts(vec![part_id.to_owned()]);
        let model = self.runtime.model();
        let scope = self.runtime.scope().expect("opened fixture scope");
        let context = objects::component_context_for_finding_part(&model, part_id)
            .expect("finding target remains a live Part");
        *self.context_override.borrow_mut() = Some(objects::ScopedTreeContext {
            scope,
            context: context.clone(),
        });
        let mut selection_kind = self
            .selection_kind
            .borrow()
            .expect("mounted selection-kind signal");
        if let Some(kind) = super::layout_workspace::selection_kind_for_tree_context(&context) {
            selection_kind.set(kind);
        }
    }

    /// Apply a real unrelated edit through the Session and CoreEngine: a project
    /// parameter plus a move of the selected part's Y, accepted and saved.
    async fn accept_unrelated_revision(&self) -> u64 {
        let accepted = self
            .runtime
            .model()
            .accepted
            .expect("fixture snapshot")
            .clone();
        let mut document = (*accepted.document).clone();
        document
            .parameters
            .insert("unrelated-inspector-edit".into(), serde_json::json!(42));
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "selected-part")
            .expect("selected fixture part")
            .pose
            .at
            .y += 1.0;
        boardstudio_web_test_runtime::project_name_test_support::submit_fixed_command(
            &self.runtime,
            boardstudio_core::model::EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "unrelated-inspector-edit".into(),
                phase: boardstudio_core::model::EditPhase::Commit,
                target_ids: vec![document.id.clone()],
                operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            },
        );
        boardstudio_web_test_runtime::project_name_test_support::run_pending(&self.runtime).await;
        self.refresh();
        self.runtime
            .model()
            .accepted
            .expect("unrelated edit accepted")
            .document
            .revision
    }

    /// Force a rerender of the mounted host after pending effects ran.
    fn refresh(&self) {
        let mut generation = self
            .render_generation
            .borrow()
            .expect("mounted render-generation signal");
        generation += 1;
    }
}

#[component]
fn mounted_component_inspector_host() -> Element {
    let probe = use_context::<MountedProbe>();
    let render_generation = use_signal(|| 0u64);
    let _ = render_generation();
    *probe.render_generation.borrow_mut() = Some(render_generation);
    let workspace = use_signal(|| "Layout");
    let selection_kind = use_signal(objects::LayoutSelectionKind::default);
    let inspect_open = use_signal(|| true);
    let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let inspector_tab = use_signal(super::layout_workspace::LayoutInspectorTab::default);
    let mut show_component = use_signal(|| true);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 1u64);
    let adapter =
        use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    let lifetime = use_hook(|| std::rc::Rc::new(LayoutComponentInspectorLifetime::default()));
    let pending_edits =
        use_signal(super::layout_component_edits::LayoutComponentInspectorEdits::default);
    *probe.pending_edits.borrow_mut() = Some(pending_edits);
    let x_draft = use_signal(String::new);
    let y_draft = use_signal(String::new);
    let margin_draft = use_signal(String::new);
    let x_failure = use_signal(|| None::<String>);
    let y_failure = use_signal(|| None::<String>);
    let margin_failure = use_signal(|| None::<String>);
    let error = use_signal(|| None::<String>);
    let layout_pending = use_signal(|| false);
    let constraint_pending = use_signal(|| false);
    let remove_pending = use_signal(|| false);
    let mirrored_half_pending = use_signal(|| false);
    let runtime = probe.runtime.clone();
    let model = runtime.model();
    let next_context = probe
        .context_override
        .borrow()
        .clone()
        .filter(|selected| {
            runtime.scope().as_ref() == Some(&selected.scope)
                && selection::context_is_current(&model, &selected.scope, &selected.context)
        })
        .or_else(|| {
            probe.selected_part.borrow().as_deref().and_then(|part_id| {
                let scope = runtime.scope()?;
                let context = objects::context_for_part(&model, part_id)?;
                Some(objects::ScopedTreeContext { scope, context })
            })
        });
    if selected_context.peek().clone() != next_context {
        selected_context.set(next_context.clone());
    }
    super::layout_workspace::use_contextual_inspector_tab_reset(selected_context, inspector_tab);
    *probe.adapter_context.borrow_mut() = selected_context.peek().clone();
    let owner_key =
        layout_component_inspector_owner_key(&model, workspace(), next_context.as_ref());
    let generation = lifetime.update(owner_key);
    let mut projection = layout_component_inspector_projection(
        &model,
        next_context.as_ref(),
        generation,
        scope_generation(),
    );
    if let (Some(projection), Some(mirrored_half)) = (
        projection.as_mut(),
        probe.mirrored_half_override.borrow().clone(),
    ) {
        projection.mirrored_half = Some(mirrored_half);
    }
    let layout_owner = current_layout_owner(&runtime, workspace, &adapter);
    let action_runtime = runtime.clone();
    let action_adapter = adapter.clone();
    let action_lifetime = lifetime.clone();
    let action_workspace = workspace;
    use boardstudio_web_ui_shared::draft_edits::FieldView;
    let views = layout_component_edits::ComponentEditViews {
        x: FieldView::new(x_draft, x_failure),
        y: FieldView::new(y_draft, y_failure),
        margin: FieldView::new(margin_draft, margin_failure),
        actions: (
            layout_pending,
            constraint_pending,
            remove_pending,
            mirrored_half_pending,
        ),
    };
    let last_action = probe.last_action.clone();
    let action_handler = EventHandler::new(move |action: LayoutComponentInspectorAction| {
        *last_action.borrow_mut() = Some(action.clone());
        dispatch_layout_component_inspector_action(
            &action_runtime,
            &action_adapter,
            &layout_owner,
            &action_lifetime,
            (action_workspace, inspect_open),
            (pending_edits, views),
            action,
        )
    });
    *probe.projection.borrow_mut() = projection.clone();
    *probe.selection_kind.borrow_mut() = Some(selection_kind);
    *probe.action.borrow_mut() = Some(action_handler);
    let switch_probe = probe.clone();
    let restore_probe = probe.clone();
    let component_probe = probe.clone();
    let clear_probe = probe.clone();
    rsx! {
        style { {include_str!("../../assets/workbench-base.css")} }
        button { id: "component-inspector-select-source", onclick: { let mut generation = render_generation; move |_| { switch_probe.select(Some("source-part")); generation += 1; } }, "Select source" }
        button { id: "component-inspector-select-original", onclick: { let mut generation = render_generation; move |_| { restore_probe.select(Some("selected-part")); generation += 1; } }, "Select original" }
        button { id: "component-inspector-select-component", onclick: { let mut generation = render_generation; move |_| { let part = component_probe.selected_part.borrow().clone().unwrap_or_else(|| "selected-part".into()); component_probe.select_component_from_finding(&part); generation += 1; } }, "Select component from finding" }
        button { id: "component-inspector-clear", onclick: { let mut generation = render_generation; move |_| { clear_probe.select(None); generation += 1; } }, "Clear selection" }
        button { id: "component-inspector-toggle-view", onclick: move |_| show_component.set(!show_component()), "Toggle Inspector view" }
        if let Some(projection) = projection {
            if show_component() {
                LayoutComponentInspector { projection, inspector_tab, pending_edits, x: x_draft, y: y_draft, margin: margin_draft, x_failure, y_failure, margin_failure, error, layout_pending, constraint_pending, remove_pending, mirrored_half_pending, on_action: action_handler }
            }
        }
        output { "data-test-render-generation": "", "{render_generation()}" }
    }
}

/// The fixture document the runtime opens: a board, two positioned parts, two empty
/// matrices for the mirrored layouts, and the outline/relationship metadata the
/// projection tests read. Core owns revisions from here on.
fn runtime_document(configure: impl FnOnce(&mut ProjectDoc)) -> ProjectDoc {
    fixture_document(|document| {
        for matrix_id in ["matrix", "paired-matrix"] {
            document.matrices.push(Matrix {
                id: matrix_id.into(),
                name: Some(matrix_id.into()),
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.0, y: 19.0 },
                origin: Vec2::default(),
                definition_id: "component-definition".into(),
                part_ids: vec![],
                board_id: Some("board".into()),
                mirror: None,
                rotation: None,
                edge_gap: None,
                diode_direction: None,
                row_layout: vec![],
                column_layout: vec![],
                anchor: None,
                cells: vec![MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 0,
                    column: 0,
                    enabled: false,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                }],
            });
        }
        configure(document);
    })
}

fn configure_mirrored_part_pair(document: &mut ProjectDoc) {
    document.boards[0].part_ids = vec!["selected-part".into()];
    document.boards.push(Board {
        id: "mirror-board".into(),
        name: "Mirror board".into(),
        outline_ids: vec![],
        part_ids: vec!["source-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "source-part")
        .unwrap()
        .pose
        .at
        .x = -4.0;
    document.layouts.clear();
    document.matrices.clear();
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology":"split", "transport":"wireless",
            "instances":[
                {"id":"source-instance","name":"Left half","boardId":"board","half":"left","role":"primary","flipped":false,"controllerPartId":null,"mechanical":null},
                {"id":"mirror-instance","name":"Right half","boardId":"mirror-board","half":"right","role":"secondary","flipped":true,"controllerPartId":null,"mechanical":null}
            ],
            "pairs":[{
                "id":"pair/A","members":["source-instance","mirror-instance"],
                "mapping":{"at":{"x":0.0,"y":0.0},"rotation":0.0},
                "construction":{"method":"printed","mount":"tray","integratedPlateFrame":false,"plateThickness":1.5,"plateFoamThickness":0.0,"bottomFoamThickness":0.0,"bottomThickness":1.5,"plateToPcb":5.0,"wallThickness":2.0,"clearance":0.2},
                "constructionSource":"source-instance",
                "halves":{"sourceBoardId":"board","mirrorBoardId":"mirror-board","correspondences":[
                    {"kind":"part","sourceId":"selected-part","mirrorId":"source-part"}
                ]}
            }]
        }))
        .expect("the mirrored board pair is valid"),
    );
}

/// A runtime whose Core requests run through the in-process engine and whose saves stay
/// in memory, opened on the fixture document with the given parts selected.
async fn mounted_runtime_probe(
    root_id: &'static str,
    configure: impl FnOnce(&mut ProjectDoc),
    selected_parts: &[&str],
) -> (MountedProbe, web_sys::Element) {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(
        &runtime,
        boardstudio_application::Session::new(boardstudio_core::CoreEngine::validate_open),
        boardstudio_core::CoreEngine::new(),
    );
    boardstudio_web_test_runtime::project_name_test_support::install_memory_persistence(&runtime);
    let document = runtime_document(configure);
    let open = runtime.operation();
    runtime.submit(boardstudio_application::Event::Open {
        operation_id: open,
        document,
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    runtime.model().accepted.expect(
        "the mounted Inspector fixture must be accepted by Session/Core Open before selection",
    );
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: selected_parts.iter().map(|id| (*id).into()).collect(),
        range_part_ids: Vec::new(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    let probe = MountedProbe {
        runtime,
        selected_part: std::rc::Rc::new(RefCell::new(Some(
            selected_parts
                .first()
                .map(|id| (*id).to_owned())
                .unwrap_or_default(),
        ))),
        context_override: std::rc::Rc::default(),
        adapter_context: std::rc::Rc::default(),
        selection_kind: std::rc::Rc::default(),
        projection: std::rc::Rc::default(),
        action: std::rc::Rc::default(),
        pending_edits: std::rc::Rc::default(),
        render_generation: std::rc::Rc::default(),
        mirrored_half_override: std::rc::Rc::default(),
        last_action: std::rc::Rc::default(),
        root_id,
    };
    let document_element = web_sys::window().unwrap().document().unwrap();
    let root = document_element.create_element("div").unwrap();
    root.set_id(probe.root_id);
    document_element
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(mounted_component_inspector_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

async fn mounted_probe(root_id: &'static str) -> (MountedProbe, web_sys::Element) {
    mounted_runtime_probe(root_id, |_| {}, &["selected-part"]).await
}

async fn mounted_probe_configured(
    root_id: &'static str,
    configure: impl FnOnce(&mut ProjectDoc),
) -> (MountedProbe, web_sys::Element) {
    mounted_runtime_probe(root_id, configure, &["selected-part"]).await
}

async fn mounted_group_probe(root_id: &'static str) -> (MountedProbe, web_sys::Element) {
    mounted_runtime_probe(
        root_id,
        |document| document.layouts[0].part_ids.clear(),
        &["selected-part", "source-part"],
    )
    .await
}

async fn settle_component_inspector(root_id: &str, mut generation: Signal<u64>) {
    generation += 1;
    let expected = generation.peek().to_string();
    wait_until(
        "the mounted component Inspector reflects its render generation",
        || {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(&format!("#{} [data-test-render-generation]", root_id))
                .ok()
                .flatten()
                .is_some_and(|label| label.text_content().as_deref() == Some(expected.as_str()))
        },
    )
    .await;
}

async fn settle_mounted_component_inspector(probe: &MountedProbe) {
    wait_until("the mounted component Inspector host renders", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{} [data-test-render-generation]", probe.root_id))
            .unwrap()
            .is_some()
    })
    .await;
    let generation =
        { (*probe.render_generation.borrow()).expect("mounted render-generation signal") };
    settle_component_inspector(probe.root_id, generation).await;
}

fn component_inspector_control(root_id: &str, role: &str, name: &str) -> web_sys::HtmlElement {
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(root_id)
        .expect("mounted Inspector root");
    by_role_name(&root, role, name).unwrap_or_else(|| panic!("{role} {name} is rendered"))
}

fn click_component_inspector(root_id: &str, role: &str, name: &str) {
    component_inspector_control(root_id, role, name).click();
}

/// Ticket 12: the source layout and its mirror partner with a stored gap,
/// plus a mirrored controller pair. The partner pose is the half-frame
/// reflection of the source pose (stored x negates).
fn configure_gapped_part_mirror(document: &mut ProjectDoc) {
    let link = document
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "paired-layout")
        .expect("mirror layout")
        .mirror_link
        .as_mut()
        .expect("mirror link");
    link.axis_x = 40.0;
    link.split_gap = Some(40.0);
    link.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: "selected-part".into(),
        mirror_id: "selected-part-mirror".into(),
    });
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: "selected-part-mirror".into(),
        definition_id: "component-definition".into(),
        reference: "U3".into(),
        pose: Pose2 {
            at: Vec2 { x: -4.0, y: 3.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.boards[0]
        .part_ids
        .push("selected-part-mirror".into());
    document
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "paired-layout")
        .expect("mirror layout")
        .part_ids
        .push("selected-part-mirror".into());
}

/// Whether the mounted Inspector has settled on the latest accepted revision.
fn accept_pending_ready(probe: &MountedProbe) -> bool {
    probe.pending_edits.borrow().as_ref().is_none_or(|edits| {
        use super::layout_component_edits::InspectorField;
        !edits.peek().is_pending(&InspectorField::PartMirror)
    })
}

/// The position of a fixture part in the accepted document.
fn accepted_position(probe: &MountedProbe, part_id: &str) -> Vec2 {
    probe
        .runtime
        .model()
        .accepted
        .expect("fixture snapshot")
        .document
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .unwrap_or_else(|| panic!("{part_id} is in the accepted document"))
        .pose
        .at
}

fn accepted_revision(probe: &MountedProbe) -> u64 {
    probe
        .runtime
        .model()
        .accepted
        .expect("fixture snapshot")
        .document
        .revision
}

/// Drive any held effects to completion so accepted state reflects every submitted edit.
async fn accept_pending(probe: &MountedProbe) {
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
    probe.refresh();
    settle_mounted_component_inspector(probe).await;
}

fn position_input(root_id: &str, label: &str) -> web_sys::HtmlInputElement {
    component_inspector_control(root_id, "spinbutton", label)
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
}

/// Type `value` into a position field and try every commit boundary the
/// Inspector listens to (Enter, blur, focusout) without relying on focus, so a
/// disabled field is exercised through the same handlers.
fn try_commit_position(input: &web_sys::HtmlInputElement, value: &str) {
    input.set_value(value);
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    for name in ["blur", "focusout"] {
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict(name, &bubbling).unwrap())
            .unwrap();
    }
}

async fn choose_inspector_option(root: &web_sys::Element, name: &str, option: &str) {
    let control = by_role_name(root, "button", name).expect("named Select trigger");
    control.click();
    wait_until("the actual choice popup opens", || {
        control.get_attribute("aria-expanded").as_deref() == Some("true")
            && by_role_name(root, "option", option).is_some()
    })
    .await;
    let option = by_role_name(root, "option", option).unwrap();
    // The primitive selects on a complete pointer gesture, not a synthetic click.
    for phase in ["pointerdown", "pointerup"] {
        let pointer = web_sys::PointerEvent::new(phase).unwrap();
        pointer.init_event_with_bubbles_and_cancelable(phase, true, true);
        option.dispatch_event(&pointer).unwrap();
    }
    wait_until("the chosen option closes the popup", || {
        control.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
}

/// The Offset X draft and visible axes/source choices shown by the copied controls.
fn offset_form(root_id: &str) -> (String, String, String) {
    let source = component_inspector_control(root_id, "combobox", "Constraint source part")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value();
    (
        position_input(root_id, "Offset X (mm)").value(),
        component_inspector_control(root_id, "button", "Offset axes")
            .text_content()
            .unwrap_or_default(),
        source,
    )
}

async fn choose_offset_axes_in_form(root_id: &str, option: &str) {
    let trigger = component_inspector_control(root_id, "button", "Layout constraint");
    if trigger.get_attribute("aria-expanded").as_deref() != Some("true") {
        trigger.click();
        wait_until("the constraint form is visible", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
    }
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(root_id)
        .unwrap();
    choose_inspector_option(&root, "Offset axes", option).await;
}

/// The Properties form takes its source, axes and numbers from `offset_drafts` on every
/// path: the first render, a change of selection, an accepted change of the saved constraint
/// (a conversion and its undo) and the reset when Properties is shown again.
// ---- Matrix Relationships section: the real Layout Inspector over a real Session ----
mod matrix_relations_panel;

fn companion_click(root: &web_sys::Element, role: &str, name: &str) {
    by_role_name(root, role, name)
        .unwrap_or_else(|| panic!("{role} {name} is rendered"))
        .click();
}

fn companion_placement_component_host() -> Element {
    let projection = use_context::<inspector::CompanionPlacementProjection>();
    let actions = use_context::<Rc<RefCell<Option<inspector::CompanionPlacementAction>>>>();
    let pending = use_signal(|| false);
    let error = use_signal(|| Some("Core refused this edit.".to_owned()));
    let pending_edits = use_signal(layout_component_edits::LayoutComponentInspectorEdits::default);
    rsx! {
        inspector::CompanionPlacementInspector {
            projection,
            pending,
            error,
            pending_edits,
            on_apply: EventHandler::new(move |action| *actions.borrow_mut() = Some(action)),
        }
    }
}

fn companion_placement_runtime_host() -> Element {
    let projection = use_context::<inspector::CompanionPlacementProjection>();
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let refuse_as_invalid_pair_tweak = use_context::<Rc<std::cell::Cell<bool>>>();
    let generation = use_signal(|| 0u64);
    let _ = generation();
    use_hook(|| {
        runtime.subscribe(Rc::new(move || {
            let mut generation = generation;
            generation += 1;
        }));
    });
    let pending = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let pending_edits = use_signal(layout_component_edits::LayoutComponentInspectorEdits::default);
    let action_runtime = runtime.clone();
    let action_edits = pending_edits;
    rsx! {
        inspector::CompanionPlacementInspector {
            projection,
            pending,
            error,
            pending_edits,
            on_apply: EventHandler::new(move |mut action: inspector::CompanionPlacementAction| {
                if refuse_as_invalid_pair_tweak.get() {
                    action.targets.push(boardstudio_core::model::CompanionPlacementTarget::Pair);
                    action.tweak = Some(boardstudio_core::model::MirroredCompanionTweak {
                        x: 0.5,
                        y: 0.25,
                        rotation: 2.0,
                    });
                }
                submit_layout_component_edit(
                    &action_runtime,
                    action_edits,
                    layout_component_edits::InspectorField::CompanionPlacement,
                    layout_component_edits::companion_placement_resolver(
                        action.pair_id,
                        action.targets,
                        action.mode,
                        action.tweak,
                    ),
                    boardstudio_web_ui_shared::draft_edits::EditDisplay::default().disabling(pending),
                );
            }),
        }
    }
}

fn companion_placement_component_projection(
    disabled_reason: Option<&'static str>,
) -> inspector::CompanionPlacementProjection {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "companion-placement-doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    inspector::CompanionPlacementProjection {
        owner: inspector::CompanionPlacementOwner {
            scope: scope.clone(),
            context: objects::ScopedTreeContext {
                scope,
                context: objects::TreeContext::Component {
                    part_id: Some("matrix/source/r0c0/led".into()),
                    matrix_id: Some("source".into()),
                    row: Some(0),
                    column: Some(0),
                    assembly_id: Some("led".into()),
                },
            },
            selected_part_ids: vec!["matrix/source/r0c0/led".into()],
            snapshot_token: SnapshotToken(13),
            revision: 9,
            context_generation: 7,
            scope_generation: 7,
        },
        pair_id: "pair/A".into(),
        targets: if disabled_reason.is_some() {
            vec![]
        } else {
            vec![boardstudio_core::model::CompanionPlacementTarget::Item {
                item_kind: boardstudio_core::model::MirroredItemKind::Part,
                mirror_id: "matrix/mirror/r0c0/led".into(),
            }]
        },
        mode: Some(boardstudio_core::model::MirroredCompanionMode::LiteralMirror),
        mixed_mode: false,
        tweak: None,
        mixed_tweak: false,
        disabled_reason,
    }
}

mod controls;
mod mounted;
mod projection;
