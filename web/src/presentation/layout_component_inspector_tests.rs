use super::inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorProjection,
};
use super::*;
use boardstudio_application::{AcceptedSnapshot, ReadModel, Scope, SessionEpoch};
use boardstudio_core::model::{
    Board, Constraint, Layout, LayoutMirrorLink, Matrix, MatrixCell, MatrixScene, MatrixSceneCell,
    PartDefinition, PartKind, PartOutline, Pose2, ProjectDoc, Readiness, SceneDelta, Side,
};
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn object_tree_context_projects_the_matching_layout_selection_mode() {
    use objects::{LayoutSelectionKind as Kind, TreeContext};

    let cases = [
        (
            TreeContext::Matrix {
                matrix_id: "matrix".into(),
            },
            Kind::Matrix,
        ),
        (
            TreeContext::Row {
                matrix_id: "matrix".into(),
                row: 1,
            },
            Kind::Row,
        ),
        (
            TreeContext::Column {
                matrix_id: "matrix".into(),
                column: 2,
            },
            Kind::Column,
        ),
        (
            TreeContext::Key {
                matrix_id: "matrix".into(),
                row: 1,
                column: 2,
            },
            Kind::Key,
        ),
        (
            TreeContext::Component {
                part_id: Some("matrix-part".into()),
                matrix_id: Some("matrix".into()),
                row: Some(1),
                column: Some(2),
                assembly_id: None,
            },
            Kind::Part,
        ),
        (
            TreeContext::Component {
                part_id: Some("standalone-part".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
            Kind::Part,
        ),
    ];

    for (context, expected) in cases {
        assert_eq!(
            super::layout_selection_kind_for_tree_context(&context),
            Some(expected),
            "{context:?} must synchronize the Select mode"
        );
    }
    for context in [
        TreeContext::Board {
            board_id: "board".into(),
        },
        TreeContext::Outline {
            board_id: "board".into(),
        },
        TreeContext::LayoutGroup {
            board_id: "board".into(),
            layout_ids: vec!["layout".into()],
        },
    ] {
        assert_eq!(
            super::layout_selection_kind_for_tree_context(&context),
            None,
            "non-selection tree contexts leave Select mode unchanged"
        );
    }
}

fn fixture() -> (ReadModel, objects::ScopedTreeContext) {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "inspector-doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let mut document = ProjectDoc::empty("inspector-doc", "Inspector fixture");
    document.revision = 9;
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
            locked: Some(id == "selected-part"),
            properties: None,
            generator_parameters: None,
        });
    }
    document.layouts.push(Layout {
        id: "source-layout".into(),
        name: "Left half".into(),
        board_id: "board".into(),
        matrix_id: "matrix".into(),
        part_ids: vec!["selected-part".into()],
        mirror_link: None,
    });
    document.layouts.push(Layout {
        id: "paired-layout".into(),
        name: "Right half".into(),
        board_id: "board".into(),
        matrix_id: "paired-matrix".into(),
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "source-layout".into(),
            axis_x: 0.0,
        }),
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

fn matrix_primary_fixture() -> (ReadModel, objects::ScopedTreeContext) {
    let (mut model, selected) = fixture();
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let document = Arc::make_mut(&mut snapshot.document);
    document.matrices.push(Matrix {
        id: "component-matrix".into(),
        name: Some("keys".into()),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "component-definition".into(),
        part_ids: vec!["selected-part".into()],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: vec![MatrixCell {
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("component-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    });
    snapshot.scene = Arc::new(SceneDelta {
        matrix_scenes: vec![MatrixScene {
            matrix_id: "component-matrix".into(),
            cells: vec![MatrixSceneCell {
                row: 0,
                column: 0,
                enabled: true,
                member_id: Some("selected-part".into()),
                pose: Pose2 {
                    at: Vec2 { x: 4.0, y: 3.0 },
                    rotation: 0.0,
                },
            }],
            columns: vec![],
        }],
        ..(*snapshot.scene).clone()
    });
    let ordinary_context = objects::context_for_part(&model, "selected-part")
        .expect("matrix primary remains selectable in Key mode");
    assert!(matches!(ordinary_context, objects::TreeContext::Key { .. }));
    let context = objects::component_context_for_finding_part(&model, "selected-part")
        .expect("an explicit Part finding has a component Inspector context");
    (
        model,
        objects::ScopedTreeContext {
            scope: selected.scope,
            context,
        },
    )
}

#[wasm_bindgen_test]
fn standalone_component_projection_preserves_accepted_owner_and_metadata() {
    let (model, selected) = fixture();
    let projection = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("selected standalone component should have an Inspector");
    assert_eq!(projection.reference, "U1");
    assert_eq!(projection.definition_name, "Fixture controller");
    assert_eq!(projection.definition_kind, "controller");
    assert_eq!(
        projection.envelope_notice.as_deref(),
        Some("Imported courtyard is approximate.")
    );
    assert!(projection.locked);
    assert_eq!(projection.layout_id.as_deref(), Some("source-layout"));
    assert_eq!(projection.owner.scope, selected.scope);
    assert_eq!(projection.owner.snapshot_token, SnapshotToken(13));
    assert_eq!(projection.owner.revision, 9);
    assert_eq!(projection.owner.context_generation, 7);
    assert_eq!(projection.owner.scope_generation, 3);
    assert_eq!(projection.outline.margin, Some(2.5));
    assert!(projection.outline.allow_body_overhang);
    assert!(projection.relationship_summary.contains("Right half"));
}

#[wasm_bindgen_test]
fn component_projection_rejects_ambiguous_and_non_standalone_selection() {
    let (mut model, selected) = fixture();
    model.selected_part_ids.push("source-part".into());
    assert!(layout_component_inspector_projection(&model, Some(&selected), 7, 3).is_none());

    let (model, mut selected) = fixture();
    selected.context = objects::TreeContext::Component {
        part_id: Some("selected-part".into()),
        matrix_id: Some("matrix".into()),
        row: None,
        column: None,
        assembly_id: None,
    };
    assert!(layout_component_inspector_projection(&model, Some(&selected), 7, 3).is_none());
}

#[wasm_bindgen_test]
fn relations_summary_uses_layout_membership_before_constraint_source() {
    let (mut model, selected) = fixture();
    {
        let snapshot = model.accepted.as_mut().expect("fixture snapshot");
        Arc::make_mut(&mut snapshot.document)
            .constraints
            .push(Constraint::Offset {
                id: "constraint".into(),
                source_part_id: "source-part".into(),
                target_part_id: "selected-part".into(),
                offset: Vec2 { x: 2.0, y: 0.0 },
                rotation: 0.0,
            });
    }
    let assigned = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("assigned component should project");
    assert!(assigned.relationship_summary.contains("Right half"));

    Arc::make_mut(&mut model.accepted.as_mut().expect("fixture snapshot").document).layouts[0]
        .part_ids
        .clear();
    let unassigned = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("unassigned component should project");
    assert!(unassigned.relationship_summary.contains("U2 drives U1"));
}

#[derive(Clone)]
struct MountedProbe {
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    model: std::rc::Rc<RefCell<ReadModel>>,
    selected_part: std::rc::Rc<RefCell<Option<String>>>,
    context_override: std::rc::Rc<RefCell<Option<objects::ScopedTreeContext>>>,
    selection_kind: std::rc::Rc<RefCell<Option<Signal<objects::LayoutSelectionKind>>>>,
    projection: std::rc::Rc<RefCell<Option<LayoutComponentInspectorProjection>>>,
    action: std::rc::Rc<RefCell<Option<EventHandler<LayoutComponentInspectorAction>>>>,
    root_id: &'static str,
}

impl MountedProbe {
    fn select(&self, part_id: Option<&str>) {
        *self.selected_part.borrow_mut() = part_id.map(str::to_owned);
        self.context_override.borrow_mut().take();
        let mut model = self.model.borrow_mut();
        model.selected_part_ids = part_id.into_iter().map(str::to_owned).collect();
        model.selection_anchor_id = part_id.map(str::to_owned);
        self.runtime.set_layout_component_inspector_test_state(
            model.clone(),
            model.accepted.as_ref().map(|snapshot| Scope {
                session_epoch: snapshot.session_epoch,
                document_id: snapshot.document.id.clone(),
                board_id: model.active_board_id.clone(),
                instance_id: model.active_instance_id.clone(),
            }),
        );
    }

    fn select_component_from_finding(&self, part_id: &str) {
        *self.selected_part.borrow_mut() = Some(part_id.to_owned());
        let mut model = self.model.borrow_mut();
        model.selected_part_ids = vec![part_id.to_owned()];
        model.selection_anchor_id = Some(part_id.to_owned());
        let snapshot = model.accepted.as_ref().expect("fixture snapshot");
        let scope = Scope {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            board_id: model.active_board_id.clone(),
            instance_id: model.active_instance_id.clone(),
        };
        let context = objects::component_context_for_finding_part(&model, part_id)
            .expect("finding target remains a live Part");
        *self.context_override.borrow_mut() = Some(objects::ScopedTreeContext {
            scope: scope.clone(),
            context: context.clone(),
        });
        let selection_kind = self
            .selection_kind
            .borrow()
            .expect("mounted selection-kind signal");
        update_layout_selection_kind_for_tree_context(selection_kind, &context);
        self.runtime
            .set_layout_component_inspector_test_state(model.clone(), Some(scope));
    }

    fn accept_unrelated_revision(&self) -> u64 {
        let mut model = self.model.borrow_mut();
        let snapshot = model.accepted.as_mut().expect("fixture snapshot");
        let document = Arc::make_mut(&mut snapshot.document);
        document.revision += 1;
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
        let revision = document.revision;
        Arc::make_mut(&mut snapshot.scene).revision = revision;
        snapshot.token = SnapshotToken(snapshot.token.0 + 1);
        let scope = Scope {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            board_id: model.active_board_id.clone(),
            instance_id: model.active_instance_id.clone(),
        };
        self.runtime
            .set_layout_component_inspector_test_state(model.clone(), Some(scope));
        revision
    }
}

#[component]
fn mounted_component_inspector_host() -> Element {
    let probe = use_context::<MountedProbe>();
    let render_generation = use_signal(|| 0u64);
    let _ = render_generation();
    let workspace = use_signal(|| "Layout");
    let selection_kind = use_signal(objects::LayoutSelectionKind::default);
    let inspector_tab = use_signal(super::layout_workspace::LayoutInspectorTab::default);
    let inspect_open = use_signal(|| true);
    let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 1u64);
    let adapter =
        use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    let lifetime = use_hook(|| std::rc::Rc::new(LayoutComponentInspectorLifetime::default()));
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
    let owner_key =
        layout_component_inspector_owner_key(&model, workspace(), next_context.as_ref());
    let generation = lifetime.update(owner_key);
    let projection = layout_component_inspector_projection(
        &model,
        next_context.as_ref(),
        generation,
        scope_generation(),
    );
    let layout_owner = current_layout_owner(&runtime, workspace, &adapter);
    let action_runtime = runtime.clone();
    let action_adapter = adapter.clone();
    let action_lifetime = lifetime.clone();
    let action_workspace = workspace;
    let action_handler = EventHandler::new(move |action| {
        dispatch_layout_component_inspector_action(
            &action_runtime,
            &action_adapter,
            &layout_owner,
            &action_lifetime,
            action_workspace,
            inspect_open,
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
    let refresh_probe = probe.clone();
    rsx! {
        style { {include_str!("../../assets/m1.css")} }
        button { id: "component-inspector-select-source", onclick: { let mut generation = render_generation; move |_| { switch_probe.select(Some("source-part")); generation += 1; } }, "Select source" }
        button { id: "component-inspector-select-original", onclick: { let mut generation = render_generation; move |_| { restore_probe.select(Some("selected-part")); generation += 1; } }, "Select original" }
        button { id: "component-inspector-select-component", onclick: { let mut generation = render_generation; move |_| { component_probe.select_component_from_finding("selected-part"); generation += 1; } }, "Select component from finding" }
        button { id: "component-inspector-clear", onclick: { let mut generation = render_generation; move |_| { clear_probe.select(None); generation += 1; } }, "Clear selection" }
        button { id: "component-inspector-unrelated-refresh", onclick: { let mut generation = render_generation; move |_| { refresh_probe.accept_unrelated_revision(); generation += 1; } }, "Accept unrelated revision" }
        if let Some(projection) = projection {
            LayoutComponentInspector { projection, inspector_tab, on_action: action_handler }
        }
    }
}

fn mounted_probe(root_id: &'static str) -> (MountedProbe, web_sys::Element) {
    let runtime = crate::runtime::Runtime::new().expect("browser runtime fixture initializes");
    let (mut model, context) = fixture();
    Arc::make_mut(&mut model.accepted.as_mut().unwrap().document)
        .parts
        .iter_mut()
        .find(|part| part.id == "selected-part")
        .unwrap()
        .locked = Some(false);
    model.selected_part_ids = vec!["selected-part".into()];
    let scope = context.scope.clone();
    mounted_probe_with_model(root_id, runtime, model, scope)
}

fn mounted_matrix_primary_probe(root_id: &'static str) -> (MountedProbe, web_sys::Element) {
    let runtime = crate::runtime::Runtime::new().expect("browser runtime fixture initializes");
    let (mut model, selected) = matrix_primary_fixture();
    Arc::make_mut(&mut model.accepted.as_mut().unwrap().document)
        .parts
        .iter_mut()
        .find(|part| part.id == "selected-part")
        .unwrap()
        .locked = Some(false);
    let scope = selected.scope;
    mounted_probe_with_model(root_id, runtime, model, scope)
}

fn mounted_probe_with_model(
    root_id: &'static str,
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    model: ReadModel,
    scope: Scope,
) -> (MountedProbe, web_sys::Element) {
    runtime.set_layout_component_inspector_test_state(model.clone(), Some(scope));
    let probe = MountedProbe {
        runtime,
        model: std::rc::Rc::new(RefCell::new(model)),
        selected_part: std::rc::Rc::new(RefCell::new(Some("selected-part".into()))),
        context_override: std::rc::Rc::default(),
        selection_kind: std::rc::Rc::default(),
        projection: std::rc::Rc::default(),
        action: std::rc::Rc::default(),
        root_id,
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(probe.root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_component_inspector_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

#[wasm_bindgen_test]
async fn mounted_matrix_key_stays_key_until_explicit_finding_opens_component_inspector() {
    let (probe, root) = mounted_matrix_primary_probe("layout-matrix-part-inspector-test-root");
    settle_component_inspector().await;
    assert_eq!(
        *probe
            .selection_kind
            .borrow()
            .expect("selection mode")
            .peek(),
        objects::LayoutSelectionKind::Key,
        "ordinary matrix primary selection remains in Key mode"
    );
    assert!(probe.projection.borrow().is_none());
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} .m1-layout-component-inspector",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "ordinary matrix primary selection remains in Key mode"
    );

    click_component_inspector(probe.root_id, "#component-inspector-select-component");
    settle_component_inspector().await;
    assert_eq!(
        probe
            .projection
            .borrow()
            .as_ref()
            .map(|projection| projection.reference.as_str()),
        Some("U1"),
        "explicit finding navigation must project the selected matrix Part"
    );
    assert_eq!(
        *probe
            .selection_kind
            .borrow()
            .expect("selection mode")
            .peek(),
        objects::LayoutSelectionKind::Part,
        "explicit finding navigation publishes the matching Part toolbar mode"
    );
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} .m1-layout-component-inspector",
                probe.root_id
            ))
            .unwrap()
            .is_some(),
        "the production component Inspector must mount for an explicit live Part context"
    );
    root.remove();
}

async fn settle_component_inspector() {
    gloo_timers::future::TimeoutFuture::new(40).await;
}

fn click_component_inspector(root_id: &str, selector: &str) {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} {selector}"))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_production_handler_rejects_selection_aba_and_unmount() {
    let (probe, root) = mounted_probe("layout-component-inspector-aba-test-root");
    settle_component_inspector().await;
    let old_projection = probe.projection.borrow().clone().unwrap();
    let old_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "#component-inspector-select-source");
    settle_component_inspector().await;
    assert_eq!(*probe.selected_part.borrow(), Some("source-part".into()));
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().reference,
        "U2",
        "component selection changes must rerender the mounted Inspector"
    );
    click_component_inspector(probe.root_id, "#component-inspector-select-original");
    settle_component_inspector().await;
    assert_ne!(
        old_projection.owner.context_generation,
        probe
            .projection
            .borrow()
            .as_ref()
            .unwrap()
            .owner
            .context_generation,
        "the production owner lifetime must advance through the selection ABA"
    );
    let _ = probe.runtime.take_layout_component_inspector_test_events();
    old_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: old_projection.owner.clone(),
        axis: ComponentPositionAxis::X,
        value: 42.0,
    });
    let stale_events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        stale_events.is_empty(),
        "stale action emitted {stale_events:?}"
    );

    let current = probe.projection.borrow().clone().unwrap();
    probe
        .action
        .borrow()
        .as_ref()
        .unwrap()
        .call(LayoutComponentInspectorAction::SetPosition {
            owner: current.owner,
            axis: ComponentPositionAxis::X,
            value: 42.0,
        });
    assert!(matches!(
        probe.runtime.take_layout_component_inspector_test_events().as_slice(),
        [boardstudio_application::Event::Edit { command, .. }]
            if matches!(&command.operation, boardstudio_core::model::EditOperation::MoveParts { positions }
                if positions.len() == 1 && positions[0].at.x == 42.0)
    ));

    let stale = probe.projection.borrow().clone().unwrap();
    let stale_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "#component-inspector-clear");
    settle_component_inspector().await;
    stale_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: stale.owner,
        axis: ComponentPositionAxis::X,
        value: 43.0,
    });
    assert!(
        probe
            .runtime
            .take_layout_component_inspector_test_events()
            .is_empty()
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_margin_enter_commits_and_escape_restores_the_accepted_value() {
    let (probe, root) = mounted_probe("layout-component-inspector-margin-test-root");
    settle_component_inspector().await;
    let _ = probe.runtime.take_layout_component_inspector_test_events();
    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    settle_component_inspector().await;
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{} .m1-layout-component-outline", probe.root_id))
            .unwrap()
            .unwrap()
            .has_attribute("open"),
        "the margin field must be in the expanded outline section"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let input = document
        .query_selector(&format!(
            "#{} input[aria-label='Part edge margin']",
            probe.root_id
        ))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.focus().unwrap();
    assert_eq!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Part edge margin"),
        "the visible margin field should receive focus before keyboard input"
    );
    input.set_value("4.25");
    let input_init = web_sys::EventInit::new();
    input_init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_init).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    assert_ne!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Part edge margin"),
        "Enter must blur the margin field so the normal commit runs"
    );
    settle_component_inspector().await;
    let margin_events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        matches!(
            margin_events.as_slice(),
            [boardstudio_application::Event::Edit { command, .. }]
                if matches!(&command.operation, boardstudio_core::model::EditOperation::ReplaceDocument { document }
                    if document.parts.iter().find(|part| part.id == "selected-part").and_then(|part| part.outline.as_ref()).and_then(|outline| outline.margin) == Some(4.25))
        ),
        "margin Enter emitted {margin_events:?}"
    );

    input.focus().unwrap();
    input.set_value("8.0");
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_init).unwrap())
        .unwrap();
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
    settle_component_inspector().await;
    assert_eq!(input.value(), "2.5");
    assert!(
        probe
            .runtime
            .take_layout_component_inspector_test_events()
            .is_empty()
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_drafts_survive_unrelated_acceptance_and_blur_uses_latest_owner() {
    let (probe, root) = mounted_probe("layout-component-inspector-refresh-test-root");
    settle_component_inspector().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let input = |label: &str| {
        document
            .query_selector(&format!("#{} input[aria-label='{label}']", probe.root_id))
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    };
    let type_value = |field: &web_sys::HtmlInputElement, value: &str| {
        field.focus().unwrap();
        field.set_value(value);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        field
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
    };

    type_value(&input("X mm"), "7.25");
    type_value(&input("Y mm"), "99.0");
    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    settle_component_inspector().await;
    type_value(&input("Part edge margin"), "5.5");
    click_component_inspector(probe.root_id, ".m1-layout-component-constraint summary");
    settle_component_inspector().await;
    type_value(&input("Offset X (mm)"), "8.5");
    click_component_inspector(probe.root_id, "button[role='tab']:nth-child(2)");
    settle_component_inspector().await;

    let accepted_revision = probe
        .model
        .borrow()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .revision
        + 1;
    let initial_context_generation = probe
        .projection
        .borrow()
        .as_ref()
        .unwrap()
        .owner
        .context_generation;
    click_component_inspector(probe.root_id, "#component-inspector-unrelated-refresh");
    settle_component_inspector().await;
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().owner.revision,
        accepted_revision
    );
    assert_eq!(
        probe
            .projection
            .borrow()
            .as_ref()
            .unwrap()
            .owner
            .context_generation,
        initial_context_generation,
        "accepted revision changes refresh the capture without ending the selected component lifetime"
    );
    assert_eq!(
        document
            .query_selector(&format!(
                "#{} button[role='tab']:nth-child(2)",
                probe.root_id
            ))
            .unwrap()
            .unwrap()
            .get_attribute("aria-selected")
            .as_deref(),
        Some("true"),
        "the selected Relations tab must survive unrelated accepted revisions"
    );
    click_component_inspector(probe.root_id, "button[role='tab']:nth-child(1)");
    settle_component_inspector().await;
    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    settle_component_inspector().await;
    assert_eq!(input("X mm").value(), "7.25");
    assert_eq!(input("Y mm").value(), "4.00");
    assert_eq!(input("Part edge margin").value(), "5.5");
    assert_eq!(input("Offset X (mm)").value(), "8.5");
    assert!(
        document
            .query_selector(&format!(
                "#{} .m1-layout-component-constraint",
                probe.root_id
            ))
            .unwrap()
            .unwrap()
            .has_attribute("open"),
        "the constraint editor disclosure state must survive an unrelated accepted revision"
    );

    let _ = probe.runtime.take_layout_component_inspector_test_events();
    let margin_field = input("Part edge margin");
    margin_field.focus().unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    margin_field
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    settle_component_inspector().await;
    let events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        matches!(
            events.as_slice(),
            [boardstudio_application::Event::Edit { command, .. }]
                if command.base_revision == accepted_revision
                    && matches!(&command.operation, boardstudio_core::model::EditOperation::ReplaceDocument { document }
                        if document.parts.iter().find(|part| part.id == "selected-part").and_then(|part| part.outline.as_ref()).and_then(|outline| outline.margin) == Some(5.5)
                            && document.parts.iter().find(|part| part.id == "selected-part").is_some_and(|part| part.pose.at.y == 4.0)
                            && document.parameters.get("unrelated-inspector-edit") == Some(&serde_json::json!(42)))
        ),
        "blur must commit the preserved draft against the latest accepted base and document: {events:?}"
    );
    root.remove();
}

#[component]
fn matrix_context_tab_reset_host() -> Element {
    let (matrix_context, key_context) = use_hook(|| {
        let (_, selected_key) = matrix_primary_fixture();
        let matrix_context = objects::ScopedTreeContext {
            scope: selected_key.scope.clone(),
            context: objects::TreeContext::Matrix {
                matrix_id: "component-matrix".into(),
            },
        };
        (matrix_context, selected_key)
    });
    let mut selected_context = use_signal(|| Some(matrix_context.clone()));
    let mut inspector_tab = use_signal(super::layout_workspace::LayoutInspectorTab::default);
    super::layout_workspace::use_contextual_inspector_tab_reset(selected_context, inspector_tab);
    let selected_label = selected_context()
        .map(|context| format!("{:?}", context.context))
        .unwrap_or_default();
    rsx! {
        button {
            id: "matrix-context-tabs-select-relations",
            onclick: move |_| inspector_tab.set(super::layout_workspace::LayoutInspectorTab::Relations),
            "Select Relations"
        }
        button {
            id: "matrix-context-tabs-select-key",
            onclick: move |_| selected_context.set(Some(key_context.clone())),
            "Select Key"
        }
        div { role: "tablist", aria_label: "Inspector details",
            button { role: "tab", aria_selected: "{inspector_tab() == super::layout_workspace::LayoutInspectorTab::Properties}", "Properties" }
            button { role: "tab", aria_selected: "{inspector_tab() == super::layout_workspace::LayoutInspectorTab::Relations}", "Relations" }
        }
        p { id: "matrix-context-tabs-selected", "{selected_label}" }
    }
}

#[wasm_bindgen_test]
async fn matrix_to_key_selection_resets_relations_tab_to_properties() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("matrix-context-tabs-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(matrix_context_tab_reset_host);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    settle_component_inspector().await;
    click_component_inspector(root.id().as_str(), "#matrix-context-tabs-select-relations");
    settle_component_inspector().await;
    assert_eq!(
        document
            .query_selector("#matrix-context-tabs-test-root [role='tab'][aria-selected='true']")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Relations"),
    );
    click_component_inspector(root.id().as_str(), "#matrix-context-tabs-select-key");
    settle_component_inspector().await;
    assert_eq!(
        document
            .query_selector("#matrix-context-tabs-test-root [role='tab'][aria-selected='true']")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Properties"),
        "a changed Matrix→Key context resets the shared Inspector tab to Properties"
    );
    root.remove();
}
