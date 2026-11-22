//! Shared fixtures and request builders for the native Core integration tests.

#[path = "support/mod.rs"]
mod shared;
#[allow(
    unused_imports,
    reason = "only some shards use the browser-shared fixtures"
)]
pub(crate) use shared::*;

pub(crate) mod fixture {
    use boardstudio_core::{CoreEngine, model::*};
    use serde::Serialize;
    use serde_json::{Value, json};

    /// Send the public JSON request, including deliberately malformed model inputs.
    pub fn request(engine: &mut CoreEngine, input: Value) -> Value {
        serde_json::from_str(&engine.request(&input.to_string())).unwrap()
    }

    pub fn wire_open(engine: &mut CoreEngine, id: &str, document: &impl Serialize) -> Value {
        request(
            engine,
            json!({"kind": "open", "id": id, "document": document}),
        )
    }

    pub fn wire_commit(
        engine: &mut CoreEngine,
        id: &str,
        transaction_id: &str,
        revision: u64,
        operation: Value,
    ) -> Value {
        request(
            engine,
            json!({
                "kind": "edit", "id": id, "command": {
                    "baseRevision": revision, "transactionId": transaction_id,
                    "phase": "commit", "targetIds": [], "operation": operation
                }
            }),
        )
    }

    pub fn wire_snapshot(engine: &mut CoreEngine, id: &str) -> Value {
        request(engine, json!({"kind": "snapshot", "id": id}))
    }

    pub fn mechanical(board_id: &str) -> MechanicalConfiguration {
        // Explicitly empty closure mounts prevent unrelated edits from initializing them.
        serde_json::from_value(json!({
            "boardId": board_id, "method": "printed", "mount": "rigid",
            "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
            "bottomFoamThickness": 0.4, "batteryHeight": 4.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
            "bottomStyle": "shell", "middleFrame": false, "openingAllowance": 0.25,
            "mounts": [], "closureMounts": [], "profiles": []
        }))
        .unwrap()
    }

    /// Normalized generator catalogue entry used when a mechanical edit needs a drill.
    pub fn mounting_hole() -> PartDefinition {
        serde_json::from_value(json!({
            "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        }))
        .unwrap()
    }

    pub fn board(id: impl Into<String>, name: impl Into<String>) -> Board {
        Board {
            id: id.into(),
            name: name.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    pub fn part_definition(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: PartKind,
    ) -> PartDefinition {
        PartDefinition {
            id: id.into(),
            name: name.into(),
            kind,
            hardware_profile: None,
            input_profile: None,
            mechanical_profile: None,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
        }
    }

    pub fn part(
        id: impl Into<String>,
        definition_id: impl Into<String>,
        reference: impl Into<String>,
    ) -> Part {
        Part {
            id: id.into(),
            definition_id: definition_id.into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            anchor: None,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    pub fn matrix(id: impl Into<String>, rows: u32, columns: u32) -> Matrix {
        Matrix {
            id: id.into(),
            rows,
            columns,
            name: None,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: None,
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: vec![],
        }
    }

    pub fn cell(row: u32, column: u32) -> MatrixCell {
        MatrixCell {
            row,
            column,
            enabled: true,
            shift: None,
            bend: None,
            deleted: false,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }
    }

    #[track_caller]
    pub fn scene_and_document(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
        match reply {
            CoreReply::Scene {
                scene, document, ..
            } => (scene, *document),
            other => panic!("expected scene: {other:?}"),
        }
    }

    #[track_caller]
    pub fn scene(reply: CoreReply) -> SceneDelta {
        scene_and_document(reply).0
    }

    #[track_caller]
    pub fn document(reply: CoreReply) -> ProjectDoc {
        scene_and_document(reply).1
    }

    pub fn open(document: ProjectDoc) -> CoreRequest {
        CoreRequest::Open {
            id: "open".into(),
            document,
        }
    }

    pub fn edit(
        id: impl Into<String>,
        transaction_id: impl Into<String>,
        base_revision: u64,
        phase: EditPhase,
        operation: EditOperation,
    ) -> CoreRequest {
        CoreRequest::Edit {
            id: id.into(),
            command: EditCommand {
                base_revision,
                transaction_id: transaction_id.into(),
                phase,
                target_ids: vec![],
                operation,
            },
        }
    }

    pub fn commit(
        id: impl Into<String>,
        transaction_id: impl Into<String>,
        base_revision: u64,
        operation: EditOperation,
    ) -> CoreRequest {
        edit(
            id,
            transaction_id,
            base_revision,
            EditPhase::Commit,
            operation,
        )
    }

    pub fn set_matrix(matrix: Matrix) -> EditOperation {
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        }
    }

    pub fn matrix_document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("p", "Project");
        doc.definitions.push(PartDefinition {
            courtyard: vec![
                Vec2 { x: -7.0, y: -7.0 },
                Vec2 { x: 7.0, y: -7.0 },
                Vec2 { x: 7.0, y: 7.0 },
                Vec2 { x: -7.0, y: 7.0 },
            ],
            ..part_definition("switch", "Switch", PartKind::Switch)
        });
        doc.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            id: "edge".into(),
            part_ids: vec![],
            settings: Default::default(),
            margin: 3.0,
            operation: Operation::Add,
        });
        doc.boards.push(Board {
            outline_ids: vec!["edge".into()],
            ..board("board", "Board")
        });
        doc
    }

    pub fn outline_document() -> ProjectDoc {
        let mut document: ProjectDoc =
            serde_json::from_str(include_str!("fixtures/synthetic-outline.json")).unwrap();
        document.format_version = boardstudio_core::document_format::CURRENT_VERSION;
        document
    }

    pub fn key_part(id: &str, x: f64, y: f64) -> Part {
        Part {
            pose: Pose2 {
                at: Vec2 { x, y },
                rotation: 0.0,
            },
            ..part(id, "switch", id)
        }
    }

    pub fn part_pose(doc: &ProjectDoc, id: &str) -> Pose2 {
        doc.parts
            .iter()
            .find(|part| part.id == id)
            .unwrap_or_else(|| panic!("missing part {id}"))
            .pose
    }

    #[track_caller]
    pub fn part_at(doc: &ProjectDoc, id: &str) -> Vec2 {
        part_pose(doc, id).at
    }

    #[track_caller]
    pub fn outline_snapshot(engine: &mut CoreEngine) -> ProjectDoc {
        document(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into(),
        }))
    }
}
