use super::inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorProjection, MirroredHalfProjection,
};
use super::*;
use boardstudio_application::{AcceptedSnapshot, ReadModel, Scope, SessionEpoch};
use boardstudio_core::model::{
    Board, Constraint, ConstraintEndpoint, ConstraintRelation, Layout, LayoutMirrorLink, Matrix,
    MatrixCell, MatrixScene, MatrixSceneCell, PartDefinition, PartKind, PartOutline, Pose2,
    ProjectDoc, Readiness, SceneDelta, Side,
};
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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
            deleted: false,
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
            companions: vec![],
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
fn component_projection_supports_group_position_selection_and_rejects_invalid_context() {
    let (mut model, selected) = fixture();
    model.selected_part_ids.push("source-part".into());
    let group = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("a live mixed component selection has a group-position projection");
    assert_eq!(group.selection_count, 2);
    assert_eq!(group.owner.selected_part_ids, model.selected_part_ids);
    assert_eq!(
        group.position.x, 4.0,
        "the first selected part anchors the group"
    );

    let mut reordered = model.clone();
    reordered.selected_part_ids.reverse();
    assert!(
        layout_component_inspector_projection(&reordered, Some(&selected), 7, 3).is_none(),
        "a component context cannot act for a group whose first selected part changed"
    );

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
    let framed = Constraint::Framed {
        id: "constraint".into(),
        source: ConstraintEndpoint::Part {
            part_id: "source-part".into(),
        },
        target: ConstraintEndpoint::Part {
            part_id: "selected-part".into(),
        },
        relation: ConstraintRelation::PartOffset {
            offset: Vec2 { x: 2.0, y: -3.0 },
            rotation: 17.0,
        },
    };
    {
        let snapshot = model.accepted.as_mut().expect("fixture snapshot");
        Arc::make_mut(&mut snapshot.document)
            .constraints
            .push(framed.clone());
    }
    let assigned = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("assigned component should project");
    assert_eq!(assigned.active_constraint, Some(framed.clone()));
    assert!(assigned.relationship_summary.contains("Right half"));

    Arc::make_mut(&mut model.accepted.as_mut().expect("fixture snapshot").document).layouts[0]
        .part_ids
        .clear();
    let unassigned = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("unassigned component should project");
    assert!(unassigned.relationship_summary.contains("U2 drives U1"));
    assert_eq!(unassigned.active_constraint, Some(framed));
}

#[wasm_bindgen_test]
fn driving_constraint_lookup_finds_the_same_record_from_both_halves_and_rejects_stale_binding() {
    use serde_json::json;

    let mut document = ProjectDoc::empty("framed-query", "Framed query");
    document.parts = serde_json::from_value(json!([
        { "id": "alpha/source", "definitionId": "part", "reference": "U1",
          "side": "front", "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "alpha/target", "definitionId": "part", "reference": "U2",
          "side": "front", "pose": { "at": { "x": 1.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "beta/source", "definitionId": "part", "reference": "U3",
          "side": "front", "pose": { "at": { "x": 2.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "beta/target", "definitionId": "part", "reference": "U4",
          "side": "front", "pose": { "at": { "x": 3.0, "y": 0.0 }, "rotation": 0.0 } }
    ]))
    .expect("the query fixture parts deserialize");
    document.hardware = Some(
        serde_json::from_value(json!({
            "instances": [
                { "id": "alpha-instance", "name": "A", "boardId": "alpha", "half": "left",
                  "role": "primary", "flipped": false, "controllerPartId": null, "mechanical": null },
                { "id": "beta-instance", "name": "B", "boardId": "beta", "half": "right",
                  "role": "secondary", "flipped": true, "controllerPartId": null, "mechanical": null }
            ],
            "pairs": [{
                "id": "pair/A", "members": ["alpha-instance", "beta-instance"],
                "mapping": { "at": { "x": 20.0, "y": -7.0 }, "rotation": 36.86989764584402 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 1.5, "plateToPcb": 5.0, "wallThickness": 2.0, "clearance": 0.2
                },
                "constructionSource": "alpha-instance",
                "halves": {
                    "sourceBoardId": "alpha", "mirrorBoardId": "beta",
                    "correspondences": [
                        { "kind": "part", "sourceId": "alpha/source", "mirrorId": "beta/source" },
                        { "kind": "part", "sourceId": "alpha/target", "mirrorId": "beta/target" }
                    ]
                }
            }]
        }))
        .expect("the query fixture pair deserializes"),
    );
    let endpoint = |mirror_part_id: &str, source_part_id: &str| ConstraintEndpoint::LinkedMirror {
        pair_id: "pair/A".into(),
        source_part_id: source_part_id.into(),
        mirror_part_id: mirror_part_id.into(),
    };
    let constraint = Constraint::Framed {
        id: "durable/constraint-7".into(),
        source: ConstraintEndpoint::Part {
            part_id: "alpha/source".into(),
        },
        target: endpoint("beta/target", "alpha/target"),
        relation: ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    };
    document.constraints.push(constraint.clone());

    for part_id in ["alpha/target", "beta/target"] {
        assert_eq!(
            document
                .constraint_driving_part(part_id)
                .expect("both endpoint views resolve"),
            Some(&constraint),
            "selecting {part_id} keeps the one stable record"
        );
    }

    document.constraints[0] = Constraint::Framed {
        id: "durable/constraint-7".into(),
        source: ConstraintEndpoint::Part {
            part_id: "alpha/source".into(),
        },
        target: endpoint("beta/target", "alpha/not-target"),
        relation: ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    };
    assert!(
        document.constraint_driving_part("beta/target").is_err(),
        "stale explicit endpoint bindings must refuse the lookup"
    );
}

#[wasm_bindgen_test]
fn inspector_projection_finds_a_driver_authored_against_the_selected_parts_linked_mirror() {
    use serde_json::json;

    let (mut model, selected) = fixture();
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let document = Arc::make_mut(&mut snapshot.document);
    document.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    document.boards[0].part_ids.push("driver-part".into());
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
    let mut driver = document.parts[0].clone();
    driver.id = "driver-part".into();
    driver.reference = "U3".into();
    document.parts.push(driver);
    document.hardware = Some(
        serde_json::from_value(json!({
            "instances": [
                { "id": "alpha-instance", "name": "A", "boardId": "board", "half": "left",
                  "role": "primary", "flipped": false, "controllerPartId": null, "mechanical": null },
                { "id": "beta-instance", "name": "B", "boardId": "mirror-board", "half": "right",
                  "role": "secondary", "flipped": true, "controllerPartId": null, "mechanical": null }
            ],
            "pairs": [{
                "id": "pair/A", "members": ["alpha-instance", "beta-instance"],
                "mapping": { "at": { "x": 20.0, "y": -7.0 }, "rotation": 36.86989764584402 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 1.5, "plateToPcb": 5.0, "wallThickness": 2.0, "clearance": 0.2
                },
                "constructionSource": "alpha-instance",
                "halves": {
                    "sourceBoardId": "board", "mirrorBoardId": "mirror-board",
                    "correspondences": [
                        { "kind": "part", "sourceId": "selected-part", "mirrorId": "source-part" }
                    ]
                }
            }]
        }))
        .expect("the linked pair fixture deserializes"),
    );
    document.constraints.push(Constraint::Framed {
        id: "stable-driver-id".into(),
        source: ConstraintEndpoint::Part {
            part_id: "driver-part".into(),
        },
        target: ConstraintEndpoint::LinkedMirror {
            pair_id: "pair/A".into(),
            source_part_id: "selected-part".into(),
            mirror_part_id: "source-part".into(),
        },
        relation: ConstraintRelation::MirrorLine {
            at: Vec2 { x: 12.0, y: -9.0 },
            angle: 31.0,
        },
    });
    let expected_constraint = document.constraints[0].clone();

    let projection = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("the selected authoritative part projects");
    let half = projection
        .mirrored_half
        .as_ref()
        .expect("linked half projects");
    assert_eq!(half.pair_id, "pair/A");
    assert_eq!(half.mirror_id, "source-part");
    assert_eq!(
        half.state,
        boardstudio_core::model::HalfExceptionState::Linked,
        "the source-side Inspector projects the linked mirror state"
    );
    assert_eq!(
        projection.active_constraint,
        Some(expected_constraint.clone()),
        "inspector projection follows authority through the linked-mirror target"
    );
    let local_edit = layout_component_edits::mirrored_half_state_resolver(
        "pair/A".into(),
        "source-part".into(),
        boardstudio_core::model::HalfExceptionState::Local,
    )
    .resolve(model.accepted.as_ref().unwrap());
    let boardstudio_application::Resolution::Submit(local_command) = local_edit else {
        panic!("a linked item resolves to one typed Local edit");
    };
    assert_eq!(local_command.target_ids, ["pair/A", "source-part"]);
    assert!(matches!(
        local_command.operation,
        boardstudio_core::model::EditOperation::SetHalfException {
            pair_id,
            kind: boardstudio_core::model::MirroredItemKind::Part,
            mirror_id,
            state: boardstudio_core::model::HalfExceptionState::Local,
        } if pair_id == "pair/A" && mirror_id == "source-part"
    ));

    let mut mirror_model = model.clone();
    mirror_model.active_board_id = "mirror-board".into();
    mirror_model.selected_part_ids = vec!["source-part".into()];
    mirror_model.selection_anchor_id = Some("source-part".into());
    let mut mirror_selected = selected.clone();
    mirror_selected.scope.board_id = "mirror-board".into();
    mirror_selected.context = objects::context_for_part(&mirror_model, "source-part")
        .expect("the linked counterpart has a current mirror-board context");
    let mirror_projection =
        layout_component_inspector_projection(&mirror_model, Some(&mirror_selected), 8, 3)
            .expect("the linked counterpart projects");
    let mirror_half = mirror_projection
        .mirrored_half
        .as_ref()
        .expect("the linked counterpart projects its half state");
    assert_eq!(mirror_half.pair_id, "pair/A");
    assert_eq!(mirror_half.mirror_id, "source-part");
    assert_eq!(
        mirror_half.state,
        boardstudio_core::model::HalfExceptionState::Linked,
        "selecting either half presents the same normalized mirror-item state"
    );
    assert_eq!(
        mirror_projection.active_constraint,
        Some(expected_constraint.clone()),
        "selecting the mirror half resolves the same durable relation"
    );

    let mut local_model = model.clone();
    Arc::make_mut(
        &mut local_model
            .accepted
            .as_mut()
            .expect("fixture snapshot")
            .document,
    )
    .hardware
    .as_mut()
    .unwrap()
    .pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(boardstudio_core::model::MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: boardstudio_core::model::MirroredItemKind::Part,
            mirror_id: "source-part".into(),
            state: boardstudio_core::model::MirroredExceptionState::Local,
            removed_copper: None,
        });
    assert_eq!(
        mirrored_half_projection(
            &local_model.accepted.as_ref().unwrap().document,
            "selected-part"
        )
        .unwrap()
        .state,
        boardstudio_core::model::HalfExceptionState::Local,
        "local state projects independently of linked constraint resolution"
    );

    let mut stale_model = model.clone();
    let stale_document = Arc::make_mut(
        &mut stale_model
            .accepted
            .as_mut()
            .expect("fixture snapshot")
            .document,
    );
    stale_document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences[0]
        .source_id = "removed-authority".into();
    assert!(
        layout_component_inspector_projection(&stale_model, Some(&selected), 9, 3).is_none(),
        "an invalid stored binding fails closed at the inspector projection"
    );
}

#[derive(Clone)]
struct MountedProbe {
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    selected_part: std::rc::Rc<RefCell<Option<String>>>,
    context_override: std::rc::Rc<RefCell<Option<objects::ScopedTreeContext>>>,
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
        crate::runtime::project_name_test_support::drive_pending(&self.runtime);
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
        crate::runtime::project_name_test_support::submit_fixed_command(
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
        crate::runtime::project_name_test_support::run_pending(&self.runtime).await;
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
    let inspector_tab = use_signal(super::layout_workspace::LayoutInspectorTab::default);
    let inspect_open = use_signal(|| true);
    let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
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
    let action_pending_edits = pending_edits;
    let last_action = probe.last_action.clone();
    let action_handler = EventHandler::new(move |action: LayoutComponentInspectorAction| {
        *last_action.borrow_mut() = Some(action.clone());
        dispatch_layout_component_inspector_action(
            &action_runtime,
            &action_adapter,
            &layout_owner,
            &action_lifetime,
            action_workspace,
            inspect_open,
            action_pending_edits,
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
        style { {include_str!("../../assets/m1.css")} }
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
    }
}

/// The fixture document the runtime opens: a board, two positioned parts, two empty
/// matrices for the mirrored layouts, and the outline/relationship metadata the
/// projection tests read. Core owns revisions from here on.
fn runtime_document(configure: impl FnOnce(&mut ProjectDoc)) -> ProjectDoc {
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
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
    }
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
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![MatrixCell {
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
    configure(&mut document);
    document
}

/// A runtime whose Core requests run through the in-process engine and whose saves stay
/// in memory, opened on the fixture document with the given parts selected.
async fn mounted_runtime_probe(
    root_id: &'static str,
    configure: impl FnOnce(&mut ProjectDoc),
    selected_parts: &[&str],
) -> (MountedProbe, web_sys::Element) {
    let runtime = crate::runtime::project_name_test_support::new_runtime();
    crate::runtime::project_name_test_support::install(
        &runtime,
        boardstudio_application::Session::new(),
        boardstudio_core::CoreEngine::new(),
    );
    crate::runtime::project_name_test_support::install_memory_persistence(&runtime);
    let document = runtime_document(configure);
    let open = runtime.operation();
    runtime.submit(boardstudio_application::Event::Open {
        operation_id: open,
        document,
    });
    crate::runtime::project_name_test_support::run_pending(&runtime).await;
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: selected_parts.iter().map(|id| (*id).into()).collect(),
        range_part_ids: Vec::new(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
    crate::runtime::project_name_test_support::run_pending(&runtime).await;
    let probe = MountedProbe {
        runtime,
        selected_part: std::rc::Rc::new(RefCell::new(Some(
            selected_parts
                .first()
                .map(|id| (*id).to_owned())
                .unwrap_or_default(),
        ))),
        context_override: std::rc::Rc::default(),
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
    mounted_runtime_probe(root_id, |_| {}, &["selected-part", "source-part"]).await
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
async fn mounted_mirrored_half_disclosure_dispatches_the_selected_one_shot_action() {
    let (probe, root) = mounted_probe("layout-component-mirrored-half-action-test-root").await;
    settle_component_inspector().await;
    *probe.mirrored_half_override.borrow_mut() = Some(MirroredHalfProjection {
        pair_id: "pair/A".into(),
        mirror_id: "mirror-part".into(),
        state: boardstudio_core::model::HalfExceptionState::Linked,
    });
    probe.refresh();
    settle_component_inspector().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let section = document
        .query_selector(&format!(
            "#{} .m1-layout-component-mirror-state",
            probe.root_id
        ))
        .unwrap()
        .expect("the mirrored-half disclosure mounts");
    assert!(section.text_content().unwrap().contains("Linked"));
    click_component_inspector(probe.root_id, ".m1-layout-component-mirror-state > summary");
    click_component_inspector(probe.root_id, "#component-inspector-mirrored-half-local");

    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetMirroredHalfState {
            pair_id,
            mirror_id,
            state: boardstudio_core::model::HalfExceptionState::Local,
            ..
        }) if pair_id == "pair/A" && mirror_id == "mirror-part"
    ));
    root.remove();
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
    crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
    probe.refresh();
    settle_component_inspector().await;
}

#[wasm_bindgen_test]
async fn mounted_matrix_key_stays_key_until_explicit_finding_opens_component_inspector() {
    let (probe, _root) = mounted_runtime_probe(
        "layout-matrix-part-inspector-test-root",
        |document| {
            // Core generates matrix members with its own ids; drop the standalone
            // selected part so the generated member takes over as the selection.
            document.parts.retain(|part| part.id == "source-part");
            document
                .boards
                .iter_mut()
                .for_each(|board| board.part_ids.retain(|id| id != "selected-part"));
            document.layouts.clear();
            document.layouts.push(Layout {
                id: "component-layout".into(),
                name: "Component half".into(),
                board_id: "board".into(),
                matrix_id: "component-matrix".into(),
                part_ids: vec![],
                mirror_link: None,
            });
            document.layouts.push(Layout {
                id: "component-mirror-layout".into(),
                name: "Component mirror".into(),
                board_id: "board".into(),
                matrix_id: "component-paired-matrix".into(),
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "component-layout".into(),
                    axis_x: 0.0,
                }),
            });
            document.matrices.clear();
            document.matrices.push(Matrix {
                id: "component-matrix".into(),
                name: Some("keys".into()),
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
                row_offsets: vec![],
                column_offsets: vec![],
                column_staggers: vec![],
                column_splays: vec![],
                column_origins: vec![],
                cells: vec![MatrixCell {
                    deleted: false,
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
            document.matrices.push(Matrix {
                id: "component-paired-matrix".into(),
                name: Some("component-paired-matrix".into()),
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
                row_offsets: vec![],
                column_offsets: vec![],
                column_staggers: vec![],
                column_splays: vec![],
                column_origins: vec![],
                cells: vec![MatrixCell {
                    deleted: false,
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
        },
        &["matrix/component-paired-matrix/r0c0"],
    )
    .await;
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
        Some("SW1"),
        "explicit finding navigation must project the selected matrix Part (Core numbers matrix members as switches)"
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
    _root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_production_handler_rejects_selection_aba_and_unmount() {
    let (probe, root) = mounted_probe("layout-component-inspector-aba-test-root").await;
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
    let before = accepted_position(&probe, "selected-part");
    old_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: old_projection.owner.clone(),
        axis: ComponentPositionAxis::X,
        value: 42.0,
        submitted_draft: "42".into(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        before,
        "a stale owner action changes nothing"
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
            submitted_draft: "42".into(),
        });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part").x,
        42.0,
        "the current owner's position edit lands"
    );

    let stale = probe.projection.borrow().clone().unwrap();
    let stale_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "#component-inspector-clear");
    settle_component_inspector().await;
    let cleared = accepted_position(&probe, "selected-part");
    stale_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: stale.owner,
        axis: ComponentPositionAxis::X,
        value: 43.0,
        submitted_draft: "43".into(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        cleared,
        "an action after selection clear changes nothing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_margin_enter_commits_and_escape_restores_the_accepted_value() {
    let (probe, root) = mounted_probe("layout-component-inspector-margin-test-root").await;
    settle_component_inspector().await;
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
    accept_pending(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == "selected-part")
            .and_then(|part| part.outline.as_ref())
            .and_then(|outline| outline.margin),
        Some(4.25),
        "margin Enter commits the draft to the accepted document"
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
    accept_pending(&probe).await;
    assert_eq!(
        input.value(),
        "4.25",
        "Escape restores the accepted value, which the Enter commit just moved"
    );
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .clone()
            .unwrap()
            .document
            .revision,
        accepted.document.revision,
        "Escape cancels the draft without another accepted edit"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_position_draft_survives_unrelated_acceptance_and_blur_uses_latest_owner()
{
    let (probe, root) = mounted_probe("layout-component-inspector-refresh-test-root").await;
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

    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    click_component_inspector(probe.root_id, ".m1-layout-component-constraint summary");
    settle_component_inspector().await;
    let x = input("X mm");
    type_value(&x, "7.25");

    let initial_revision = accepted_revision(&probe);
    let initial_context_generation = probe
        .projection
        .borrow()
        .as_ref()
        .unwrap()
        .owner
        .context_generation;
    // Advance accepted state without changing focus. Driving a control here would blur and
    // submit the draft before the unrelated acceptance being tested.
    let accepted_revision = probe.accept_unrelated_revision().await;
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
        "accepted revisions refresh action capture without ending component selection"
    );
    assert_eq!(
        x.value(),
        "7.25",
        "dirty focused X draft survives the refresh"
    );
    assert_eq!(
        input("Y mm").value(),
        "4.00",
        "clean Y follows the accepted position"
    );
    for selector in [
        ".m1-layout-component-outline",
        ".m1-layout-component-constraint",
    ] {
        assert!(
            document
                .query_selector(&format!("#{} {selector}", probe.root_id))
                .unwrap()
                .unwrap()
                .has_attribute("open"),
            "disclosure {selector} survives unrelated acceptance"
        );
    }

    let _ = x.blur();
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 7.25, y: 4.0 },
        "blur lands the retained X draft with the accepted Y against the refreshed owner"
    );
    assert!(accepted_revision > initial_revision);
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_margin_enter_uses_latest_accepted_document() {
    let (probe, root) = mounted_probe("layout-component-inspector-margin-refresh-test-root").await;
    settle_component_inspector().await;
    let document = web_sys::window().unwrap().document().unwrap();
    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    settle_component_inspector().await;
    let margin = document
        .query_selector(&format!(
            "#{} input[aria-label='Part edge margin']",
            probe.root_id
        ))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    margin.focus().unwrap();
    margin.set_value("5.5");
    let input = web_sys::EventInit::new();
    input.set_bubbles(true);
    margin
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input).unwrap())
        .unwrap();

    probe.accept_unrelated_revision().await;
    settle_component_inspector().await;
    assert_eq!(
        margin.value(),
        "5.5",
        "dirty margin draft survives acceptance"
    );
    assert_eq!(
        document
            .query_selector(&format!("#{} input[aria-label='Y mm']", probe.root_id))
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
            .value(),
        "4.00",
        "clean position follows accepted state"
    );
    assert!(
        document
            .query_selector(&format!("#{} .m1-layout-component-outline", probe.root_id))
            .unwrap()
            .unwrap()
            .has_attribute("open"),
        "outline disclosure survives unrelated acceptance"
    );

    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    margin
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    accept_pending(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap();
    let selected = accepted
        .document
        .parts
        .iter()
        .find(|part| part.id == "selected-part")
        .unwrap();
    assert_eq!(
        selected.outline.as_ref().and_then(|outline| outline.margin),
        Some(5.5),
        "Enter commits the margin against the latest accepted document"
    );
    assert_eq!(
        selected.pose.at.y, 4.0,
        "the unrelated accepted move survives the margin edit"
    );
    assert_eq!(
        accepted.document.parameters.get("unrelated-inspector-edit"),
        Some(&serde_json::json!(42)),
        "the unrelated accepted parameter survives the margin edit"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_relations_tab_survives_unrelated_acceptance() {
    let (probe, root) = mounted_probe("layout-component-inspector-tab-refresh-test-root").await;
    settle_component_inspector().await;
    let document = web_sys::window().unwrap().document().unwrap();
    click_component_inspector(probe.root_id, "button[role='tab']:nth-child(2)");
    settle_component_inspector().await;
    let revision = probe.accept_unrelated_revision().await;
    settle_component_inspector().await;
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().owner.revision,
        revision
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
        "Relations tab survives an unrelated accepted revision"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_uses_constraint_and_navigation_copy() {
    let (probe, root) = mounted_probe("layout-component-inspector-copy-test-root").await;
    settle_component_inspector().await;
    let document = web_sys::window().unwrap().document().unwrap();

    let constraint_type = document
        .query_selector(&format!(
            "#{} .m1-layout-component-constraint select",
            probe.root_id
        ))
        .unwrap()
        .expect("the constraint selector mounts")
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap();
    assert_eq!(
        constraint_type.get_attribute("aria-label").as_deref(),
        Some("Constraint type")
    );
    assert!(
        document
            .query_selector(&format!(
                "#{} .m1-layout-component-constraint label",
                probe.root_id
            ))
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap_or_default()
            .contains("Constraint type"),
        "the visible constraint label is: {}",
        root.text_content().unwrap_or_default()
    );

    click_component_inspector(probe.root_id, "button[role='tab']:nth-child(2)");
    settle_component_inspector().await;
    let navigation = document
        .query_selector(&format!(
            "#{} .m1-layout-component-inspector button:not([role='tab'])",
            probe.root_id
        ))
        .unwrap()
        .expect("the Relations navigation button mounts");
    assert_eq!(
        navigation.text_content().unwrap_or_default(),
        "Go to Properties"
    );
    navigation
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle_component_inspector().await;
    assert_eq!(
        document
            .query_selector(&format!(
                "#{} button[role='tab']:nth-child(1)",
                probe.root_id
            ))
            .unwrap()
            .unwrap()
            .get_attribute("aria-selected")
            .as_deref(),
        Some("true"),
        "the navigation button opens Properties"
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

fn position_input(root_id: &str, label: &str) -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} input[aria-label='{label}']"))
        .unwrap()
        .unwrap_or_else(|| panic!("{label} input is rendered"))
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

#[wasm_bindgen_test]
async fn mounted_editable_component_position_commits_so_the_unavailable_cases_are_not_vacuous() {
    let (probe, root) = mounted_probe("layout-component-position-editable-test-root").await;
    settle_component_inspector().await;
    let x = position_input(probe.root_id, "X mm");
    assert!(!x.disabled(), "an unlocked, undriven component edits X");
    try_commit_position(&x, "42");
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part").x,
        42.0,
        "an unlocked, undriven component's X edit lands"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_rapid_xy_enter_and_focus_change_submit_one_edit_per_axis() {
    let (probe, root) =
        mounted_probe_configured("layout-component-position-rapid-xy-test-root", |document| {
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap();
            part.reference = "J1".into();
            part.pose.at = Vec2 {
                x: 66.675,
                y: -47.625,
            };
        })
        .await;
    settle_component_inspector().await;
    let initial_revision = accepted_revision(&probe);
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);

    x.focus().unwrap();
    x.set_value("60");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();

    // Moving focus to Y causes the native X blur after Enter. It must not submit
    // the same still-visible X draft a second time.
    y.focus().unwrap();
    y.set_value("-40");
    y.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    y.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();

    accept_pending(&probe).await;
    assert_eq!(
        accepted_revision(&probe),
        initial_revision + 2,
        "rapid X Enter followed by X blur and Y Enter must accept exactly one edit per axis"
    );
    assert_eq!(
        accepted_position(&probe, "selected-part").y,
        -40.0,
        "the Y edit landed"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_rapid_xy_queued_edits_keep_both_coordinates_and_undo_removes_only_y() {
    let (probe, root) = mounted_probe_configured(
        "layout-component-position-queued-xy-test-root",
        |document| {
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap();
            part.reference = "J1".into();
            part.pose.at = Vec2 {
                x: 66.675,
                y: -47.625,
            };
        },
    )
    .await;
    settle_component_inspector().await;
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);

    // Hold the first Core reply so the Y edit queues behind the X edit, exactly as
    // rapid typing does when the engine is busy.
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);

    x.focus().unwrap();
    x.set_value("60");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered
        .await
        .expect("the X edit reached the in-process Core");

    y.focus().unwrap();
    y.set_value("-40");
    y.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    y.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    release.send(()).expect("release the held X reply");
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 60.0, y: -40.0 },
        "queued X then Y edits must both survive: the Y edit resolves against the document the X edit produced"
    );

    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 {
            x: 60.0,
            y: -47.625
        },
        "one Undo removes only the Y change"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_position_landing_preserves_a_newer_unsubmitted_draft() {
    let (probe, root) = mounted_probe("layout-component-position-newer-draft-test-root").await;
    settle_component_inspector().await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let x = position_input(probe.root_id, "X mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    x.focus().unwrap();
    x.set_value("60");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered.await.expect("the position edit reached Core");

    x.set_value("61");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    release.send(()).expect("release the held position result");
    accept_pending(&probe).await;
    assert_eq!(accepted_position(&probe, "selected-part").x, 60.0);
    assert_eq!(
        x.value(),
        "61",
        "landing the older edit must preserve newer typed text"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_view_unmount_retires_held_field_observation_and_remount_refreshes()
 {
    let (probe, root) = mounted_probe("layout-component-position-unmount-test-root").await;
    settle_component_inspector().await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let x = position_input(probe.root_id, "X mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    x.focus().unwrap();
    x.set_value("60");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered.await.expect("the position edit reached Core");

    click_component_inspector(probe.root_id, "#component-inspector-toggle-view");
    settle_component_inspector().await;
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} [aria-label='Component inspector']",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "the view-only toggle unmounts the Inspector"
    );
    assert_eq!(*probe.selected_part.borrow(), Some("selected-part".into()));
    click_component_inspector(probe.root_id, "#component-inspector-toggle-view");
    settle_component_inspector().await;
    release
        .send(())
        .expect("release the held position result after remount");
    accept_pending(&probe).await;
    assert_eq!(accepted_position(&probe, "selected-part").x, 60.0);
    assert_eq!(position_input(probe.root_id, "X mm").value(), "60.00");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_failed_save_reverts_the_field_with_an_inline_message() {
    let (probe, root) = mounted_probe("layout-component-position-save-failure-test-root").await;
    settle_component_inspector().await;
    crate::runtime::project_name_test_support::fail_next_persist(
        &probe.runtime,
        "injected durable write failure",
    );
    let x = position_input(probe.root_id, "X mm");
    x.focus().unwrap();
    x.set_value("7.5");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 4.0, y: 3.0 },
        "a failed save must not move the part"
    );
    settle_component_inspector().await;
    assert_eq!(
        x.value(),
        "4.00",
        "a failed edit shows the accepted value again"
    );
    let alert = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} [role='alert']", probe.root_id))
        .unwrap()
        .expect("the failure is explained inline")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("did not save") && alert.contains("injected durable write failure"),
        "the inline message names the failure: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn queued_y_edit_waits_for_recovery_after_x_save_failure() {
    let (probe, root) = mounted_probe("layout-component-queued-edit-recovery-test-root").await;
    settle_component_inspector().await;
    crate::runtime::project_name_test_support::fail_next_persist(
        &probe.runtime,
        "injected X write failure",
    );
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);

    x.focus().unwrap();
    x.set_value("8");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered.await.expect("the X edit reached Core");

    y.focus().unwrap();
    y.set_value("9");
    y.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    let y_outcome = crate::runtime::project_name_test_support::observe_next(&probe.runtime);
    y.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    assert!(
        probe
            .pending_edits
            .borrow()
            .expect("the mounted Layout Inspector owns pending edits")
            .peek()
            .is_pending(&layout_component_edits::InspectorField::Y),
        "the Y edit is admitted while the X reply is held"
    );
    release.send(()).expect("release the held X reply");
    // The held X reply resumes a spawned task. Refresh this manually driven host
    // only after that task has published the queued Y edit's terminal outcome.
    for _ in 0..100 {
        if y_outcome.borrow().is_some() {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(1).await;
    }
    assert_eq!(
        *y_outcome.borrow(),
        Some(boardstudio_application::TerminalOutcome::BlockedByRecovery(
            "injected X write failure".into()
        )),
        "the admitted Y edit fails because the X save requires recovery"
    );
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 4.0, y: 3.0 },
        "the queued Y edit cannot land while the failed save requires recovery"
    );
    assert_eq!(
        probe.runtime.model().lifecycle,
        boardstudio_application::Lifecycle::RecoveryRequired,
        "a durable save failure requires reopening the accepted document before another edit"
    );
    assert_eq!(
        y.value(),
        "3.00",
        "the blocked Y draft restores its accepted value"
    );
    let alert = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} [role='alert']", probe.root_id))
        .unwrap()
        .expect("the X failure remains visible")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("injected X write failure"),
        "unexpected alert: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn successful_y_landing_preserves_an_earlier_x_resolver_failure() {
    let (probe, root) = mounted_probe("layout-component-independent-field-errors-test-root").await;
    settle_component_inspector().await;
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    x.focus().unwrap();
    x.set_value("8");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();

    let pending = probe
        .pending_edits
        .borrow()
        .expect("the mounted Layout Inspector owns pending edits")
        .peek()
        .clone();
    pending.begin_field(
        &probe.runtime,
        layout_component_edits::InspectorField::X,
        "layout-inspector-x",
        Some("layout".into()),
        layout_component_edits::position_resolver(
            "departed-part".into(),
            vec!["departed-part".into()],
            "board".into(),
            ComponentPositionAxis::X,
            8.0,
        ),
        "8",
    );
    crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
    probe.refresh();
    settle_component_inspector().await;
    assert_eq!(
        probe.runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Ready,
        "a resolver retirement is a field failure without entering save recovery"
    );
    let x_failure = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} [role='alert']", probe.root_id))
        .unwrap()
        .expect("the X resolver failure is visible inline")
        .text_content()
        .unwrap();
    assert!(
        x_failure.contains("selected part no longer exists"),
        "unexpected X failure: {x_failure}"
    );

    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    y.focus().unwrap();
    y.set_value("9");
    y.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    y.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 4.0, y: 9.0 },
        "the valid Y field lands independently of X's resolver failure"
    );
    let alert = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} [role='alert']", probe.root_id))
        .unwrap()
        .expect("the earlier X failure remains visible")
        .text_content()
        .unwrap();
    assert_eq!(
        alert, x_failure,
        "Y landing must not clear X's inline failure"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_edit_retires_when_the_part_becomes_locked_before_execution() {
    let (probe, root) = mounted_probe("layout-component-position-retire-test-root").await;
    settle_component_inspector().await;
    // Hold the lock edit's Core reply so the position edit queues behind it.
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let accepted = probe
        .runtime
        .model()
        .accepted
        .expect("fixture snapshot")
        .clone();
    let mut replacement = (*accepted.document).clone();
    replacement
        .parts
        .iter_mut()
        .find(|part| part.id == "selected-part")
        .unwrap()
        .locked = Some(true);
    crate::runtime::project_name_test_support::submit_fixed_command(
        &probe.runtime,
        boardstudio_core::model::EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "lock-before-position".into(),
            phase: boardstudio_core::model::EditPhase::Commit,
            target_ids: vec!["selected-part".into()],
            operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
        },
    );
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered
        .await
        .expect("the lock edit reached the in-process Core");

    let x = position_input(probe.root_id, "X mm");
    x.focus().unwrap();
    x.set_value("7.5");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    release.send(()).expect("release the held lock edit");
    // The parked task drives the lock edit and the queued position edit to completion
    // on its own; only afterwards does a forced rerender settle the ticket.
    settle_component_inspector().await;
    probe.refresh();
    settle_component_inspector().await;
    assert!(
        accepted_position(&probe, "selected-part") == Vec2 { x: 4.0, y: 3.0 },
        "a retired edit moves nothing"
    );
    let alert = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} [role='alert']", probe.root_id))
        .unwrap()
        .expect("the retirement is explained inline")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("locked"),
        "the inline message explains why the edit retired: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_position_untouched_blur_and_escape_cancel_submit_no_edit() {
    let (probe, root) = mounted_probe_configured(
        "layout-component-position-untouched-blur-test-root",
        |document| {
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap();
            part.reference = "J1".into();
            part.pose.at = Vec2 {
                x: 66.675,
                y: -47.625,
            };
        },
    )
    .await;
    settle_component_inspector().await;
    let initial_revision = accepted_revision(&probe);
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);

    // Merely moving through both fields must not turn their two-decimal
    // presentation into a coordinate edit on blur.
    x.focus().unwrap();
    y.focus().unwrap();
    x.focus().unwrap();

    // Escape cancels a real draft. The ensuing blur must not submit the
    // restored, formatted display value as a replacement for raw geometry.
    x.set_value("71.125");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
    )
    .unwrap();
    let _ = x.blur();
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 {
            x: 66.675,
            y: -47.625
        },
        "untouched focus/blur and Escape-cancel must not move the part"
    );
    assert_eq!(
        accepted_revision(&probe),
        initial_revision,
        "untouched focus/blur and Escape-cancel must accept no edits"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_group_position_blur_moves_all_selected_parts_from_first_anchor() {
    let (probe, root) = mounted_group_probe("layout-component-group-position-test-root").await;
    settle_component_inspector().await;
    let document = web_sys::window().unwrap().document().unwrap();
    assert_eq!(
        document
            .query_selector(&format!(
                "#{} .m1-layout-component-selection-note",
                probe.root_id
            ))
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("2 parts selected. Position edits apply to the selection.")
    );
    let x = position_input(probe.root_id, "X mm");
    assert!(!x.disabled());
    try_commit_position(&x, "5.5");
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 5.5, y: 3.0 },
        "the group position edit moves the anchor part"
    );
    assert_eq!(
        accepted_position(&probe, "source-part"),
        Vec2 { x: 11.5, y: 3.0 },
        "the group position edit translates every selected part by the anchor delta"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_locked_component_presents_xy_unavailable_and_suppresses_commit() {
    let (probe, root) =
        mounted_probe_configured("layout-component-position-locked-test-root", |document| {
            document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap()
                .locked = Some(true);
        })
        .await;
    settle_component_inspector().await;
    assert!(probe.projection.borrow().as_ref().unwrap().locked);
    let initial_revision = accepted_revision(&probe);
    for (label, value) in [("X mm", "50"), ("Y mm", "60")] {
        let input = position_input(probe.root_id, label);
        assert!(
            input.disabled(),
            "{label} must be unavailable for a locked part"
        );
        try_commit_position(&input, value);
    }
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 4.0, y: 3.0 },
        "locked position commits never move the part"
    );
    assert_eq!(
        accepted_revision(&probe),
        initial_revision,
        "locked position commits accept no edits"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_driven_component_presents_xy_unavailable_and_suppresses_commit() {
    let (probe, root) =
        mounted_probe_configured("layout-component-position-driven-test-root", |document| {
            document.constraints.push(Constraint::Offset {
                id: "driver".into(),
                source_part_id: "source-part".into(),
                target_part_id: "selected-part".into(),
                offset: Vec2 { x: 2.0, y: 0.0 },
                rotation: 0.0,
            });
        })
        .await;
    settle_component_inspector().await;
    let projection = probe.projection.borrow().clone().unwrap();
    assert!(!projection.locked, "the part itself is not locked");
    assert!(projection.active_constraint.is_some(), "the part is driven");
    let driven = accepted_position(&probe, "selected-part");
    for (label, value) in [("X mm", "50"), ("Y mm", "60")] {
        let input = position_input(probe.root_id, label);
        assert!(
            input.disabled(),
            "{label} must be unavailable for a driven part"
        );
        try_commit_position(&input, value);
    }
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        driven,
        "driven position commits never move the part"
    );
    root.remove();
}

fn offset_form_select(root_id: &str, label: &str) -> web_sys::HtmlSelectElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} select[aria-label='{label}']"))
        .unwrap()
        .unwrap_or_else(|| panic!("the {label} choice mounts"))
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap()
}

/// The Offset X draft, the Offset axes choice and the Source part choice the form shows.
fn offset_form(root_id: &str) -> (String, String, String) {
    (
        position_input(root_id, "Offset X (mm)").value(),
        offset_form_select(root_id, "Offset axes").value(),
        offset_form_select(root_id, "Constraint source part").value(),
    )
}

fn choose_offset_axes_in_form(root_id: &str, value: &str) {
    let select = offset_form_select(root_id, "Offset axes");
    select.set_value(value);
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    select
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &bubbling).unwrap())
        .unwrap();
}

/// The Properties form takes its source, axes and numbers from `offset_drafts` on every
/// path: the first render, a change of selection, an accepted change of the saved constraint
/// (a conversion and its undo) and the reset when Properties is shown again.
#[wasm_bindgen_test]
async fn mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_the_reset() {
    const KEY: &str = "matrix/component-matrix/r0c0";
    let (probe, root) = mounted_probe_configured("layout-offset-form-test-root", |document| {
        // The key sits between the two standalone parts, so the first part that is not
        // the selected one is a key for one selection and not for the other.
        document.boards[0].part_ids =
            vec!["selected-part".into(), KEY.into(), "source-part".into()];
        let mut key = document.parts[0].clone();
        key.id = KEY.into();
        key.reference = "SW1".into();
        key.outline = None;
        key.pose.rotation = 30.0;
        document.parts.push(key);
        document.matrices.push(Matrix {
            id: "component-matrix".into(),
            name: None,
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "component-definition".into(),
            part_ids: vec![KEY.into()],
            board_id: Some("board".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![0.0],
            column_origins: vec![],
            cells: vec![MatrixCell {
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }],
        });
        document.constraints.push(Constraint::PartOffset {
            id: "held".into(),
            source_part_id: KEY.into(),
            target_part_id: "selected-part".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        });
    })
    .await;
    settle_component_inspector().await;
    let order: Vec<(String, bool)> = probe
        .projection
        .borrow()
        .as_ref()
        .expect("the selected part projects")
        .board_parts
        .iter()
        .map(|part| (part.id.clone(), part.matrix_key))
        .collect();
    assert_eq!(
        order[..3],
        [
            ("selected-part".to_owned(), false),
            (KEY.to_owned(), true),
            ("source-part".to_owned(), false),
        ],
        "the fixture's board order and key flags"
    );
    let root_id = probe.root_id;

    // First render: the saved part-offset constraint.
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "part".into(), KEY.into()),
        "first render"
    );

    // A selection change to a part without a constraint: its first other part is the
    // selected-part, which is not a key, so the default is board axes and zeros.
    click_component_inspector(root_id, "#component-inspector-select-source");
    settle_component_inspector().await;
    assert_eq!(
        offset_form(root_id),
        ("0".into(), "board".into(), "selected-part".into()),
        "after selecting an unconstrained part"
    );

    // Back to the constrained part: its saved kind, source and numbers again.
    click_component_inspector(root_id, "#component-inspector-select-original");
    settle_component_inspector().await;
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "part".into(), KEY.into()),
        "after selecting the constrained part again"
    );

    // A conversion re-expresses the numbers on the board axes at the key's 30 degree turn.
    choose_offset_axes_in_form(root_id, "board");
    accept_pending(&probe).await;
    let (x, axes, source) = offset_form(root_id);
    assert!(
        (x.parse::<f64>().unwrap() - 2.866_025_403_784_439).abs() < 1e-6,
        "converted X draft {x}"
    );
    assert_eq!((axes.as_str(), source.as_str()), ("board", KEY));

    // Undoing the conversion restores the saved part-offset constraint and its numbers.
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "part".into(), KEY.into()),
        "after undoing the conversion"
    );

    // The reset: an unconstrained part's unsaved axes choice goes back to the default when
    // Properties is shown again.
    click_component_inspector(root_id, "#component-inspector-select-source");
    settle_component_inspector().await;
    choose_offset_axes_in_form(root_id, "part");
    settle_component_inspector().await;
    assert_eq!(offset_form(root_id).1, "part", "the unsaved choice shows");
    click_component_inspector(root_id, "button[role='tab']:nth-child(2)");
    settle_component_inspector().await;
    click_component_inspector(root_id, "button[role='tab']:nth-child(1)");
    settle_component_inspector().await;
    assert_eq!(
        offset_form(root_id),
        ("0".into(), "board".into(), "selected-part".into()),
        "after the reset"
    );
    root.remove();
}

// ---- Matrix Relations tab (ticket 09): the real Layout Inspector over a real Session ----

mod matrix_relations_panel {
    use super::super::layout_workspace;
    use super::super::*;
    use super::{click_component_inspector, settle_component_inspector};
    use boardstudio_application::{Event, Session};
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{
        CoreReply, CoreRequest, EditCommand, EditPhase, ProjectDoc, Vec2,
    };
    use serde_json::json;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen_test::wasm_bindgen_test;

    const MATRIX: &str = "main";

    #[derive(Clone)]
    struct RelationsProbe {
        runtime: Rc<crate::runtime::Runtime>,
        root_id: &'static str,
        version: Rc<RefCell<Option<Signal<u64>>>>,
        selected: Rc<RefCell<Option<Signal<Option<objects::ScopedTreeContext>>>>>,
        adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    }

    impl RelationsProbe {
        fn scope(&self) -> Scope {
            self.runtime.scope().expect("opened project scope")
        }

        fn select(&self, context: objects::TreeContext) {
            let adapter = self.adapter.borrow().clone().expect("mounted adapter");
            selection::submit_context(
                &self.runtime,
                &adapter,
                selection::ContextRequest {
                    scope: self.scope(),
                    context,
                    mode: boardstudio_application::SelectionMode::Replace,
                },
            );
        }

        fn selected_context(&self) -> Option<objects::TreeContext> {
            let signal = (*self.selected.borrow())?;
            signal
                .peek()
                .as_ref()
                .map(|selected| selected.context.clone())
        }

        fn bump(&self) {
            let mut version = self.version.borrow().expect("mounted version signal");
            version += 1;
        }

        /// Drive every pending effect, re-render the host and let effects settle.
        async fn land(&self) {
            crate::runtime::project_name_test_support::run_pending(&self.runtime).await;
            settle_component_inspector().await;
            crate::runtime::project_name_test_support::run_pending(&self.runtime).await;
            self.bump();
            settle_component_inspector().await;
        }

        fn text(&self) -> String {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id(self.root_id)
                .unwrap()
                .text_content()
                .unwrap_or_default()
        }

        fn click(&self, selector: &str) {
            click_component_inspector(self.root_id, selector);
        }

        fn exists(&self, selector: &str) -> bool {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(&format!("#{} {selector}", self.root_id))
                .unwrap()
                .is_some()
        }

        async fn open_relations_tab(&self) {
            self.click(".m1-layout-component-tabs button:nth-child(2)");
            settle_component_inspector().await;
        }

        fn matrix(&self) -> boardstudio_core::model::Matrix {
            self.runtime
                .model()
                .accepted
                .expect("accepted document")
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == MATRIX)
                .cloned()
                .expect("the matrix exists")
        }
    }

    #[component]
    fn relations_host() -> Element {
        let probe = use_context::<RelationsProbe>();
        let version = use_signal(|| 0u64);
        *probe.version.borrow_mut() = Some(version);
        let _ = version();
        let workspace = use_signal(|| "Layout");
        let scope_generation = use_signal(|| 1u64);
        let selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
        *probe.selected.borrow_mut() = Some(selected_context);
        let anchor_scope = use_signal(|| None::<Scope>);
        let adapter =
            use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
        *probe.adapter.borrow_mut() = Some(adapter.clone());
        // The key actions mounted by the Inspector read the Editor's shared contexts.
        use_context_provider(|| probe.runtime.clone());
        use_context_provider(|| adapter.clone());
        use_context_provider(|| version);
        use_context_provider(|| WorkspaceState(workspace));
        let relation_highlight = use_signal(|| None::<Vec<String>>);
        use_context_provider(|| RelationHighlight(relation_highlight));
        let state = layout_workspace::use_layout_workspace_state(selected_context, workspace);
        let runtime = probe.runtime.clone();
        let lifetime = use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
        let model = runtime.model();
        let selected = selected_context();
        let owner_key =
            layout_component_inspector_owner_key(&model, workspace(), selected.as_ref());
        let generation = lifetime.update(owner_key);
        let component_inspector = layout_component_inspector_projection(
            &model,
            selected.as_ref(),
            generation,
            scope_generation(),
        );
        let matrix_inspector = objects::use_matrix_inspector(
            runtime.clone(),
            version,
            selected_context,
            workspace,
            adapter.generation,
        );
        let board_inspector = board_inspector::use_board_inspector(
            runtime.clone(),
            selected_context,
            workspace,
            adapter.generation,
        );
        let key_size = objects::use_key_size(
            runtime.clone(),
            version,
            selected_context,
            workspace,
            adapter.generation,
        );
        let transform = objects::use_workspace_matrix_transform(
            runtime.clone(),
            version,
            selected_context,
            workspace,
            adapter.generation,
            state.splay_affect(),
            "Layout",
        );
        let (x, y, margin, error) = state.component_inspector_drafts();
        let (layout_pending, constraint_pending, remove_pending, mirrored_half_pending) =
            state.component_inspector_pending_controls();
        let (x_failure, y_failure, margin_failure) = state.component_inspector_field_failures();
        let select_runtime = runtime.clone();
        let select_adapter = adapter.clone();
        rsx! {
            {layout_workspace::inspector(layout_workspace::InspectorInput {
                geometry_scripts_open: false,
                on_close_geometry_scripts: EventHandler::new(|()| {}),
                context_title: None,
                context_detail: None,
                show_position_inspector: false,
                component_inspector,
                component_inspector_pending_edits: state.component_inspector_edits(),
                component_inspector_x: x,
                component_inspector_y: y,
                component_inspector_margin: margin,
                component_inspector_error: error,
                component_inspector_x_failure: x_failure,
                component_inspector_y_failure: y_failure,
                component_inspector_margin_failure: margin_failure,
                component_inspector_layout_pending: layout_pending,
                component_inspector_constraint_pending: constraint_pending,
                component_inspector_remove_pending: remove_pending,
                component_inspector_mirrored_half_pending: mirrored_half_pending,
                on_component_inspector_action: EventHandler::new(|_| {}),
                matrix_inspector,
                key_size,
                matrix_transform_inspector: transform,
                on_pick_splay_origin: EventHandler::new(|()| {}),
                splay_origin_pick_pending: false,
                on_cancel_splay_origin_pick: EventHandler::new(|()| {}),
                inspector_tab: state.inspector_tab(),
                matrix_relations: matrix_context_relations(&model, selected.as_ref()),
                relation_target: {
                    let runtime = runtime.clone();
                    Rc::new(move |request| relation_target_request(&runtime, request))
                },
                relation_scope: selected.as_ref().map(|selected| selected.scope.clone()),
                on_select_context: EventHandler::new(move |request: objects::TreeSelectRequest| {
                    selection::submit_context(&select_runtime, &select_adapter, request.into());
                }),
                outline_inspector: None,
                findings_return_available: false,
                on_findings_return: EventHandler::new(|()| {}),
                board_inspector_mount: board_inspector,
                findings_page: None,
            })}
        }
    }

    fn definition(id: &str, kind: &str) -> serde_json::Value {
        json!({
            "id": id, "name": id, "kind": kind,
            "courtyard": [
                { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
                { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
            ],
            "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                       "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
        })
    }

    fn base() -> ProjectDoc {
        let mut document =
            serde_json::to_value(ProjectDoc::empty("relations-doc", "Relations")).unwrap();
        document["definitions"] = json!([
            definition("switch", "switch"),
            definition("diode", "passive")
        ]);
        document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
            "partIds": [], "netIds": [], "thickness": 1.6 }]);
        serde_json::from_value(document).unwrap()
    }

    fn matrix_json(rows: u32, columns: u32, extra: serde_json::Value) -> serde_json::Value {
        let mut matrix = json!({
            "id": MATRIX, "rows": rows, "columns": columns,
            "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "switch", "boardId": "board", "partIds": []
        });
        for (key, value) in extra.as_object().into_iter().flatten() {
            matrix[key] = value.clone();
        }
        matrix
    }

    fn set_matrix_command(base_revision: u64, matrix: serde_json::Value) -> EditCommand {
        EditCommand {
            base_revision,
            transaction_id: "set-matrix".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: serde_json::from_value(json!({ "kind": "set-matrix", "matrix": matrix }))
                .expect("the operation parses"),
        }
    }

    /// Open `document`, then create the matrix through the Session and Core.
    async fn mount(
        root_id: &'static str,
        document: ProjectDoc,
        matrix: Option<serde_json::Value>,
    ) -> (RelationsProbe, web_sys::Element) {
        let support = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(
            &support,
            Session::new(),
            CoreEngine::new(),
        );
        crate::runtime::project_name_test_support::install_memory_persistence(&support);
        support.submit(Event::Open {
            operation_id: support.operation(),
            document,
        });
        crate::runtime::project_name_test_support::run_pending(&support).await;
        if let Some(matrix) = matrix {
            let revision = support.model().accepted.unwrap().document.revision;
            crate::runtime::project_name_test_support::submit_fixed_command(
                &support,
                set_matrix_command(revision, matrix),
            );
            crate::runtime::project_name_test_support::run_pending(&support).await;
        }
        let probe = RelationsProbe {
            runtime: support,
            root_id,
            version: Rc::default(),
            selected: Rc::default(),
            adapter: Rc::default(),
        };
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        root.set_id(root_id);
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(relations_host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle_component_inspector().await;
        (probe, root)
    }

    fn matrix_ctx() -> objects::TreeContext {
        objects::TreeContext::Matrix {
            matrix_id: MATRIX.into(),
        }
    }
    fn row_ctx(row: u32) -> objects::TreeContext {
        objects::TreeContext::Row {
            matrix_id: MATRIX.into(),
            row,
        }
    }
    fn column_ctx(column: u32) -> objects::TreeContext {
        objects::TreeContext::Column {
            matrix_id: MATRIX.into(),
            column,
        }
    }
    fn key_ctx(row: u32, column: u32) -> objects::TreeContext {
        objects::TreeContext::Key {
            matrix_id: MATRIX.into(),
            row,
            column,
        }
    }

    const OLD_FALLBACK: &str = "No saved placement relationship on this selection.";
    const OLD_NOTE: &str = "Matrix rows and columns share pitch, stagger and splay";

    #[wasm_bindgen_test]
    async fn a_blank_matrix_shows_structure_for_the_matrix_a_row_a_column_and_a_key() {
        let (probe, root) = mount(
            "relations-blank-test",
            base(),
            Some(matrix_json(3, 4, json!({}))),
        )
        .await;
        for (context, expected) in [
            (matrix_ctx(), "3 × 4, 12 keys"),
            (row_ctx(1), "4 keys in Row 2"),
            (column_ctx(2), "0 mm from Column 2"),
            (key_ctx(0, 0), "Row 1, Column 1"),
        ] {
            probe.select(context.clone());
            probe.land().await;
            probe.open_relations_tab().await;
            let text = probe.text();
            assert!(
                probe.exists("[aria-labelledby='m1-relations-structure-heading']"),
                "{context:?} shows Structure: {text}"
            );
            assert!(text.contains(expected), "{context:?}: {text}");
            assert!(!text.contains(OLD_FALLBACK), "{context:?}: {text}");
            assert!(!text.contains(OLD_NOTE), "{context:?}: {text}");
            assert!(!probe.exists("[aria-labelledby='m1-relations-departures-heading']"));
            assert!(!probe.exists("[aria-labelledby='m1-relations-constraints-heading']"));
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_column_offset_departure_resets_with_undo_and_converts_to_a_stagger() {
        let (probe, root) = mount(
            "relations-departure-test",
            base(),
            Some(matrix_json(
                2,
                3,
                json!({ "columnOffsets": [{ "x": 0.0, "y": 0.0 }, { "x": 1.5, "y": 2.0 }] }),
            )),
        )
        .await;
        probe.select(column_ctx(1));
        probe.land().await;
        probe.open_relations_tab().await;
        assert!(probe.exists("[data-relation-departure='Column offset']"));

        probe.click("[data-relation-action='reset']");
        probe.land().await;
        assert_eq!(
            probe.matrix().column_offsets[1],
            Vec2 { x: 0.0, y: 0.0 },
            "Reset lands through SetMatrixPlacement"
        );
        assert!(
            !probe.exists("[data-relation-departure='Column offset']"),
            "the departure disappears: {}",
            probe.text()
        );

        probe.runtime.submit(Event::Undo {
            operation_id: probe.runtime.operation(),
        });
        probe.land().await;
        assert_eq!(probe.matrix().column_offsets[1], Vec2 { x: 1.5, y: 2.0 });
        assert!(
            probe.exists("[data-relation-departure='Column offset']"),
            "Undo restores the departure: {}",
            probe.text()
        );

        probe.click("[data-relation-action='convert']");
        probe.land().await;
        let matrix = probe.matrix();
        assert_eq!(matrix.column_staggers[1], 2.0);
        assert_eq!(matrix.column_offsets[1], Vec2 { x: 1.5, y: 0.0 });
        assert!(
            probe.text().contains("2 mm from Column 1, cumulative 2 mm"),
            "Structure shows the new stagger: {}",
            probe.text()
        );
        root.remove();
    }

    /// Two keys' companions, one tied to the first key by an explicit constraint. Core
    /// generates the part ids, so a scratch engine does the work and the document that
    /// results opens in the Session.
    fn constrained_companion_document() -> (ProjectDoc, String) {
        let mut engine = CoreEngine::new();
        let opened = match engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: base(),
        }) {
            CoreReply::Scene { scene, .. } => scene,
            other => panic!("{other:?}"),
        };
        let assemblies: Vec<serde_json::Value> = ["a", "b"]
            .iter()
            .enumerate()
            .map(|(index, id)| {
                json!({ "id": id, "definitionId": "diode",
                        "offset": { "x": 0.0, "y": 8.0 + index as f64 * 3.0 } })
            })
            .collect();
        let matrix = matrix_json(
            1,
            2,
            json!({ "cells": [{ "row": 0, "column": 0, "enabled": true, "assemblies": assemblies }] }),
        );
        let (scene, mut document) = match engine.handle(CoreRequest::Edit {
            id: "edit".into(),
            command: set_matrix_command(opened.revision, matrix),
        }) {
            CoreReply::Scene {
                scene, document, ..
            } => (scene, *document),
            other => panic!("{other:?}"),
        };
        let companion = scene.matrix_scenes[0].companions[1].part_id.clone();
        document
            .constraints
            .push(boardstudio_core::model::Constraint::Offset {
                id: "tie".into(),
                source_part_id: "matrix/main/r0c0".into(),
                target_part_id: companion.clone(),
                offset: Vec2 { x: 0.0, y: 12.0 },
                rotation: 0.0,
            });
        (document, companion)
    }

    #[wasm_bindgen_test]
    async fn a_constraint_reference_selects_its_object_and_escape_returns() {
        let (document, companion) = constrained_companion_document();
        let (probe, root) = mount("relations-constraint-test", document, None).await;
        probe.select(matrix_ctx());
        probe.land().await;
        probe.open_relations_tab().await;
        assert!(
            probe.exists("[data-relation-constraint='tie']"),
            "the constraint is listed: {}",
            probe.text()
        );

        probe.click("[data-relation-constraint='tie'] [data-relation-role='source']");
        probe.land().await;
        assert_eq!(probe.selected_context(), Some(key_ctx(0, 0)));

        probe.select(matrix_ctx());
        probe.land().await;
        probe.open_relations_tab().await;
        probe.click("[data-relation-constraint='tie'] [data-relation-role='target']");
        probe.land().await;
        assert!(
            matches!(
                probe.selected_context(),
                Some(objects::TreeContext::Component { part_id: Some(ref id), .. }) if *id == companion
            ),
            "the companion is selected: {:?}",
            probe.selected_context()
        );

        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        options.set_bubbles(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options).unwrap();
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .dispatch_event(&event)
            .unwrap();
        probe.land().await;
        assert!(
            matches!(
                probe.selected_context(),
                Some(objects::TreeContext::Board { .. })
            ),
            "Escape returns: {:?} text={}",
            probe.selected_context(),
            probe.text()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn changing_the_selection_shows_no_stale_rows() {
        let (probe, root) = mount(
            "relations-stale-test",
            base(),
            Some(matrix_json(2, 3, json!({}))),
        )
        .await;
        probe.select(column_ctx(1));
        probe.land().await;
        probe.open_relations_tab().await;
        assert!(probe.text().contains("Inherited by"), "{}", probe.text());

        probe.select(key_ctx(0, 1));
        probe.land().await;
        probe.open_relations_tab().await;
        let text = probe.text();
        assert!(text.contains("Pitch cell"), "{text}");
        assert!(!text.contains("Inherited by"), "{text}");
        assert!(!text.contains("Stagger"), "{text}");
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_reference_to_a_removed_key_disappears_and_a_queued_click_does_nothing() {
        let (document, _) = constrained_companion_document();
        let (probe, root) = mount("relations-removed-test", document, None).await;
        probe.select(matrix_ctx());
        probe.land().await;
        probe.open_relations_tab().await;
        let source = "[data-relation-constraint='tie'] [data-relation-role='source']";
        assert!(probe.exists(source), "{}", probe.text());

        // Remove the referenced key, letting the edit land without re-rendering the panel:
        // the button is still in the DOM, as when a click was queued before the removal.
        let accepted = probe.runtime.model().accepted.unwrap();
        crate::runtime::project_name_test_support::submit_fixed_command(
            &probe.runtime,
            EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "remove-key".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: boardstudio_core::model::EditOperation::RemoveMatrixCells {
                    matrix_id: MATRIX.into(),
                    cells: vec![(0, 0)],
                },
            },
        );
        crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
        let landed = probe.runtime.model().accepted.unwrap();
        assert!(
            landed.document.revision > accepted.document.revision,
            "the removal landed"
        );
        let before = probe.selected_context();
        probe.click(source);
        crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
        assert_eq!(
            probe.selected_context(),
            before,
            "the stale click did nothing"
        );

        probe.bump();
        settle_component_inspector().await;
        assert!(
            !probe.exists(source),
            "the removed key's row is gone: {}",
            probe.text()
        );
        root.remove();
    }
}
